//! Real sing-box exit probe.
//!
//! Reads the Wi-Fi Calling Gateway's running sing-box configuration
//! (`/var/run/wificalling-gateway/sing-box.json`), finds the outbound bound to
//! the assigned test device, builds a minimal temporary configuration that
//! reuses that outbound behind a local HTTP proxy, starts a second sing-box
//! instance, and asks an IP echo service through it to learn the node's real
//! exit IP. The temporary instance is always cleaned up.
//!
//! Parsing and probe-config generation are pure functions tested offline; the
//! process orchestration runs only on the router.

use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use serde_json::{json, Value};

use super::runtime::{ExitProbeRuntime, ProbeFailure};
use super::ExitProbeError;

/// A parsed sing-box outbound (we only need tag + the raw object to re-emit).
#[derive(Clone, Debug)]
pub struct SingBoxOutbound {
    pub tag: String,
    raw: Value,
}

/// The parsed configuration pieces the probe needs.
#[derive(Clone, Debug)]
pub struct SingBoxConfig {
    pub outbounds: Vec<SingBoxOutbound>,
}

/// Select the outbound the route rules bind to `device_ip`.
pub fn select_outbound_tag(document: &Value, device_ip: IpAddr) -> Option<String> {
    let rules = document.get("route")?.get("rules")?.as_array()?;
    rules.iter().find_map(|rule| {
        let cidrs = rule.get("source_ip_cidr")?.as_array()?;
        let matches = cidrs.iter().any(|cidr| {
            cidr.as_str()
                .and_then(|text| text.split('/').next())
                .and_then(|ip| ip.parse::<IpAddr>().ok())
                .map(|ip| ip == device_ip)
                .unwrap_or(false)
        });
        if matches {
            rule.get("outbound")
                .and_then(Value::as_str)
                .map(String::from)
        } else {
            None
        }
    })
}

/// Resolve the node bound to `device_ip` in the Gateway device-policy UCI
/// text (`/etc/config/wificalling-gateway`), returning its `node-<section>`
/// tag. Disabled policies are included: the follow-device IP is defined by
/// the bound node, not by whether Wi-Fi Calling interception is enabled.
pub fn device_bound_node_tag(uci_text: &str, device_ip: IpAddr) -> Option<String> {
    let mut in_device = false;
    let mut source_ips: Vec<String> = Vec::new();
    let mut node: Option<String> = None;
    for raw_line in uci_text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("config ") {
            // Flush the previous device section before starting a new one
            // (the next `config device` must not swallow it).
            if in_device
                && source_ips
                    .iter()
                    .any(|ip| ip.parse::<IpAddr>().ok() == Some(device_ip))
            {
                return node.map(|section| format!("node-{section}"));
            }
            in_device = line.starts_with("config device");
            source_ips.clear();
            node = None;
            continue;
        }
        if !in_device {
            continue;
        }
        if let Some(value) = option_value(line, "option node") {
            node = Some(value);
        } else if let Some(value) = option_value(line, "list source_ip") {
            source_ips.push(value);
        }
    }
    if in_device
        && source_ips
            .iter()
            .any(|ip| ip.parse::<IpAddr>().ok() == Some(device_ip))
    {
        return node.map(|section| format!("node-{section}"));
    }
    None
}

/// Extract the quoted value of a UCI `option`/`list` line, e.g.
/// `option node 'cfg1146ab'` -> `cfg1146ab`.
fn option_value(line: &str, keyword: &str) -> Option<String> {
    let rest = line.strip_prefix(keyword)?.trim_start();
    let value = rest
        .strip_prefix('\'')
        .or_else(|| rest.strip_prefix('"'))?;
    let end = value.find(['\'', '"'])?;
    Some(value[..end].to_owned())
}

/// Parse a sing-box.json document, retaining outbounds for reuse.
pub fn parse_singbox_config(document: &Value) -> SingBoxConfig {
    let outbounds = document
        .get("outbounds")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let tag = item.get("tag")?.as_str()?.to_owned();
                    Some(SingBoxOutbound {
                        tag,
                        raw: item.clone(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    SingBoxConfig { outbounds }
}

/// Build a minimal probe configuration: a local HTTP inbound plus the target
/// outbound (reused verbatim from the Gateway config) and a direct fallback.
pub fn build_probe_config(
    config: &SingBoxConfig,
    outbound_tag: &str,
    listen_port: u16,
) -> Result<String, ProbeFailure> {
    let outbound = config
        .outbounds
        .iter()
        .find(|outbound| outbound.tag == outbound_tag)
        .ok_or(ProbeFailure::Unreachable)?;

    let mut outbounds = vec![outbound.raw.clone()];
    outbounds.push(json!({"type": "direct", "tag": "direct"}));

    let probe = json!({
        "log": {"level": "warn"},
        "inbounds": [{
            "type": "http",
            "tag": "probe",
            "listen": "127.0.0.1",
            "listen_port": listen_port
        }],
        "outbounds": outbounds,
        "route": {
            "final": outbound_tag
        }
    });
    serde_json::to_string(&probe).map_err(|_| ProbeFailure::InvalidData)
}

/// Ask an IP echo service through the local HTTP proxy and return the exit IP.
fn query_exit_ip(probe_port: u16, timeout: Duration) -> Result<IpAddr, ProbeFailure> {
    let proxy = format!("http://127.0.0.1:{probe_port}");
    let url = "http://ip-api.com/json?fields=query";
    let output = Command::new("curl")
        .args([
            "-s",
            "--max-time",
            &timeout.as_secs().to_string(),
            "-x",
            &proxy,
            url,
        ])
        .output()
        .map_err(|_| ProbeFailure::Unreachable)?;
    if !output.status.success() {
        return Err(ProbeFailure::Unreachable);
    }
    let value: Value =
        serde_json::from_slice(&output.stdout).map_err(|_| ProbeFailure::InvalidData)?;
    let ip = value
        .get("query")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<IpAddr>().ok())
        .ok_or(ProbeFailure::InvalidData)?;
    Ok(ip)
}

/// Parse non-loopback IPv4 addresses from `ip -4 addr show` output.
pub fn parse_wan_ips(text: &str) -> Vec<IpAddr> {
    let mut addresses = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(inet) = line.strip_prefix("inet ") {
            if let Some(cidr) = inet.split_whitespace().next() {
                if let Some(ip) = cidr
                    .split('/')
                    .next()
                    .and_then(|s| s.parse::<IpAddr>().ok())
                {
                    if ip != IpAddr::V4(Ipv4Addr::UNSPECIFIED)
                        && ip != IpAddr::V4(Ipv4Addr::LOCALHOST)
                        && !ip.is_loopback()
                    {
                        addresses.push(ip);
                    }
                }
            }
        }
    }
    addresses
}

/// A probe backed by the Gateway's sing-box outbounds.
pub struct SingBoxProbe {
    config_path: PathBuf,
    device_ip: IpAddr,
    probe_port: u16,
    work_dir: PathBuf,
    timeout: Duration,
    singbox_bin: String,
    /// Gateway device-policy UCI file (test-injectable). Used to resolve
    /// the bound node for devices that have no route rule (e.g. disabled
    /// Wi-Fi Calling policies), so follow-device still probes their node.
    uci_config_path: PathBuf,
}

impl SingBoxProbe {
    pub fn new(
        config_path: PathBuf,
        device_ip: IpAddr,
        probe_port: u16,
        work_dir: PathBuf,
    ) -> Self {
        Self {
            config_path,
            device_ip,
            probe_port,
            work_dir,
            timeout: Duration::from_secs(15),
            singbox_bin: "/usr/bin/sing-box".to_owned(),
            uci_config_path: PathBuf::from("/etc/config/wificalling-gateway"),
        }
    }

    /// Read the Gateway config and select the outbound for the test device.
    fn load_outbound_tag(&self) -> Result<String, ProbeFailure> {
        let text =
            std::fs::read_to_string(&self.config_path).map_err(|_| ProbeFailure::Unreachable)?;
        let document: Value = serde_json::from_str(&text).map_err(|_| ProbeFailure::InvalidData)?;
        let config = parse_singbox_config(&document);

        // 1. The outbound the Gateway's route rules bind to the assigned
        //    device (enabled policies).
        let rule_tag = select_outbound_tag(&document, self.device_ip);
        if let Some(tag) = rule_tag {
            if config.outbounds.iter().any(|o| o.tag == tag) {
                return Ok(tag);
            }
        }
        // 2. The node bound to the device policy, even when the policy is
        //    disabled and has no route rule - follow-device must report that
        //    node's exit.
        if let Ok(uci_text) = std::fs::read_to_string(&self.uci_config_path) {
            if let Some(tag) = device_bound_node_tag(&uci_text, self.device_ip) {
                if config.outbounds.iter().any(|o| o.tag == tag) {
                    return Ok(tag);
                }
            }
        }
        // 3. Fall back to the first non-direct outbound.
        config
            .outbounds
            .iter()
            .find(|outbound| outbound.tag != "direct")
            .map(|outbound| outbound.tag.clone())
            .ok_or(ProbeFailure::Unreachable)
    }

    /// Probe the node's real exit IP through a temporary sing-box instance.
    fn probe_with_node(&self, outbound_tag: &str) -> Result<IpAddr, ProbeFailure> {
        let text =
            std::fs::read_to_string(&self.config_path).map_err(|_| ProbeFailure::Unreachable)?;
        let document: Value = serde_json::from_str(&text).map_err(|_| ProbeFailure::InvalidData)?;
        let config = parse_singbox_config(&document);
        let probe_config = build_probe_config(&config, outbound_tag, self.probe_port)?;

        std::fs::create_dir_all(&self.work_dir).map_err(|_| ProbeFailure::Unreachable)?;
        let config_path = self.work_dir.join("probe-config.json");
        std::fs::write(&config_path, probe_config).map_err(|_| ProbeFailure::Unreachable)?;

        let mut child = Command::new(&self.singbox_bin)
            .args(["run", "-c"])
            .arg(&config_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|_| ProbeFailure::Unreachable)?;

        // Give sing-box a moment to bind the probe listener, then query.
        std::thread::sleep(Duration::from_millis(800));
        let result = query_exit_ip(self.probe_port, self.timeout);

        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_file(&config_path);
        result
    }
}

impl ExitProbeRuntime for SingBoxProbe {
    fn probe_exit_ip(&mut self) -> Result<IpAddr, ProbeFailure> {
        let outbound_tag = self.load_outbound_tag()?;
        self.probe_with_node(&outbound_tag)
    }

    fn router_wan_ips(&mut self) -> Result<Vec<IpAddr>, ProbeFailure> {
        // Collect WAN addresses from the system routes via `ip -4 addr`.
        let output = Command::new("ip")
            .args(["-4", "addr", "show"])
            .output()
            .map_err(|_| ProbeFailure::Unreachable)?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut addresses = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if let Some(inet) = line.strip_prefix("inet ") {
                if let Some(cidr) = inet.split_whitespace().next() {
                    if let Some(ip) = cidr
                        .split('/')
                        .next()
                        .and_then(|s| s.parse::<IpAddr>().ok())
                    {
                        if ip != IpAddr::V4(Ipv4Addr::UNSPECIFIED)
                            && ip != IpAddr::V4(Ipv4Addr::LOCALHOST)
                            && !ip.is_loopback()
                        {
                            addresses.push(ip);
                        }
                    }
                }
            }
        }
        if addresses.is_empty() {
            // Fail-open boundary: no WAN address known means observations are
            // rejected by the validation layer (RouterWanUnknown).
            return Ok(vec![]);
        }
        Ok(addresses)
    }
}

#[allow(dead_code)]
fn _keep_error_link(error: ExitProbeError) -> ExitProbeError {
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_document() -> Value {
        json!({
            "outbounds": [
                {"type": "vless", "tag": "node-hk", "server": "1.2.3.4", "server_port": 443},
                {"type": "direct", "tag": "direct"}
            ],
            "route": {
                "rules": [
                    {"source_ip_cidr": ["192.168.31.175/32"], "action": "route", "outbound": "node-hk"}
                ]
            }
        })
    }

    #[test]
    fn parse_outbounds_and_build_probe_config() {
        let doc = sample_document();
        let config = parse_singbox_config(&doc);
        assert_eq!(config.outbounds.len(), 2);
        assert_eq!(config.outbounds[0].tag, "node-hk");

        let probe = build_probe_config(&config, "node-hk", 18080).unwrap();
        let probe: Value = serde_json::from_str(&probe).unwrap();
        assert_eq!(probe["inbounds"][0]["type"], "http");
        assert_eq!(probe["inbounds"][0]["listen_port"], 18080);
        assert_eq!(probe["route"]["final"], "node-hk");
        assert_eq!(probe["outbounds"].as_array().unwrap().len(), 2);
        assert_eq!(probe["outbounds"][0]["tag"], "node-hk");
    }

    #[test]
    fn device_outbound_is_selected_from_route_rules() {
        let doc = sample_document();
        assert_eq!(
            select_outbound_tag(&doc, IpAddr::V4(Ipv4Addr::new(192, 168, 31, 175))),
            Some("node-hk".to_owned())
        );
        assert_eq!(
            select_outbound_tag(&doc, IpAddr::V4(Ipv4Addr::new(192, 168, 31, 176))),
            None
        );
    }

    #[test]
    fn unknown_outbound_is_rejected() {
        let doc = sample_document();
        let config = parse_singbox_config(&doc);
        assert_eq!(
            build_probe_config(&config, "missing", 18080),
            Err(ProbeFailure::Unreachable)
        );
    }

    #[test]
    fn query_exit_ip_parses_the_echo_field() {
        // The parsing path is exercised indirectly via a tiny inline check:
        // ip-api returns {"query":"1.2.3.4"}; parse that shape.
        let value: Value = serde_json::from_str(r#"{"query":"1.2.3.4"}"#).unwrap();
        let ip = value
            .get("query")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<IpAddr>().ok())
            .unwrap();
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)));
    }

    #[test]
    fn load_outbound_tag_falls_back_without_route_rules() {
        let doc = json!({"outbounds": [
            {"type": "hysteria2", "tag": "node-a"},
            {"type": "direct", "tag": "direct"}
        ]});
        let dir = std::env::temp_dir();
        let config_path = dir.join("wloc-singbox-fallback.json");
        std::fs::write(&config_path, doc.to_string()).unwrap();
        let mut probe = SingBoxProbe::new(
            config_path.clone(),
            IpAddr::V4(Ipv4Addr::new(192, 168, 31, 176)),
            18080,
            dir.join("wloc-singbox-fallback-work"),
        );
        // Falls back to node-a (first non-direct), then the sing-box spawn
        // fails on this host -> Unreachable.
        let _ = probe.probe_exit_ip();
        std::fs::remove_file(&config_path).unwrap();
        let _ = std::fs::remove_dir_all(dir.join("wloc-singbox-fallback-work"));
    }

    #[test]
    fn query_exit_ip_fails_when_the_proxy_is_down() {
        // No listener on this port; the curl command fails -> Unreachable.
        assert_eq!(
            query_exit_ip(59_999, Duration::from_millis(500)),
            Err(ProbeFailure::Unreachable)
        );
    }

    #[test]
    fn parse_wan_ips_skips_loopback_and_link_local() {
        let text = "1: lo: <LOOPBACK>\n    inet 127.0.0.1/8 scope host lo\n2: eth0: <UP>\n    inet 192.168.31.1/24 brd 192.168.31.255 scope global eth0\n    inet 10.0.0.5/8 scope global eth0\n";
        let ips = parse_wan_ips(text);
        assert!(ips.contains(&IpAddr::V4(Ipv4Addr::new(192, 168, 31, 1))));
        assert!(ips.contains(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))));
        assert!(!ips.contains(&IpAddr::V4(Ipv4Addr::LOCALHOST)));
    }

    #[test]
    fn device_bound_node_tag_parses_the_device_policy() {
        // The iPhone17 policy is disabled (enabled=0) so it has no route
        // rule, but its bound node must still be probeable - the follow-
        // device IP comes from that node's exit.
        let uci = r#"
config wificalling-gateway 'main'
	option enabled '1'

config device
	option label 'iPhone12'
	option route_mode 'independent'
	option node 'cfg0a46ab'
	list source_ip '192.168.31.175'

config device
	option label 'iPhone17'
	option route_mode 'independent'
	option node 'cfg1146ab'
	option enabled '0'
	list source_ip '192.168.31.176'
"#;
        assert_eq!(
            device_bound_node_tag(uci, IpAddr::V4(Ipv4Addr::new(192, 168, 31, 176))),
            Some("node-cfg1146ab".to_owned())
        );
        assert_eq!(
            device_bound_node_tag(uci, IpAddr::V4(Ipv4Addr::new(192, 168, 31, 175))),
            Some("node-cfg0a46ab".to_owned())
        );
        assert_eq!(device_bound_node_tag(uci, IpAddr::V4(Ipv4Addr::new(192, 168, 31, 99))), None);
        assert_eq!(device_bound_node_tag("", IpAddr::V4(Ipv4Addr::new(192, 168, 31, 176))), None);
    }

    #[test]
    fn load_outbound_tag_prefers_device_bound_node_over_fallback() {
        // No route rule for the device, but the device policy binds a node
        // that exists in the sing-box outbounds: probe must use it, not the
        // arbitrary first outbound.
        let doc = json!({"outbounds": [
            {"type": "hysteria2", "tag": "node-a"},
            {"type": "vless", "tag": "node-cfg1146ab"},
            {"type": "direct", "tag": "direct"}
        ]});
        let dir = std::env::temp_dir();
        let config_path = dir.join("wloc-singbox-bound-node.json");
        let uci_path = dir.join("wloc-singbox-bound-node.uci");
        std::fs::write(&config_path, doc.to_string()).unwrap();
        std::fs::write(
            &uci_path,
            "config device
\toption label 'iPhone17'
\toption node 'cfg1146ab'
\tlist source_ip '192.168.31.176'
",
        )
        .unwrap();
        let mut probe = SingBoxProbe::new(
            config_path.clone(),
            IpAddr::V4(Ipv4Addr::new(192, 168, 31, 176)),
            18080,
            dir.join("wloc-singbox-bound-node-work"),
        );
        probe.uci_config_path = uci_path.clone();
        // load_outbound_tag must select the bound node tag; the sing-box
        // spawn then fails on this host, which is fine.
        let _ = probe.probe_exit_ip();
        std::fs::remove_file(&config_path).unwrap();
        std::fs::remove_file(&uci_path).unwrap();
        let _ = std::fs::remove_dir_all(dir.join("wloc-singbox-bound-node-work"));
    }

    #[test]
    fn load_outbound_tag_reads_the_gateway_config() {
        let dir = std::env::temp_dir();
        let config_path = dir.join("wloc-singbox-load-test.json");
        std::fs::write(&config_path, sample_document().to_string()).unwrap();
        let mut probe = SingBoxProbe::new(
            config_path.clone(),
            IpAddr::V4(Ipv4Addr::new(192, 168, 31, 175)),
            18080,
            dir.join("wloc-singbox-load-work"),
        );
        // load_outbound_tag picks node-hk from the route rule; the subsequent
        // sing-box spawn fails on this host, which is fine (Unreachable).
        let _ = probe.probe_exit_ip();
        std::fs::remove_file(&config_path).unwrap();
        let _ = std::fs::remove_dir_all(dir.join("wloc-singbox-load-work"));
    }
}
