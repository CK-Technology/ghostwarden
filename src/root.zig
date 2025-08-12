const std = @import("std");

pub const Config = @import("config.zig");
pub const Http = @import("http.zig");
pub const CrowdSec = @import("crowdsec.zig");
pub const Wazuh = @import("wazuh.zig");
pub const Proxmox = @import("proxmox.zig");
pub const Daemon = @import("daemon.zig");
pub const Nftables = @import("nftables.zig");
pub const Metrics = @import("metrics.zig");

pub const GhostWardenError = error{
    ConfigurationError,
    NetworkError,
    AuthenticationError,
    ApiError,
    ParseError,
};

pub fn bufferedPrint() !void {
    const stdout = std.io.getStdOut().writer();
    try stdout.print("Ghostwarden module loaded successfully\n");
}

test {
    _ = Config;
    _ = Http;
    _ = CrowdSec;
    _ = Wazuh;
    _ = Proxmox;
    _ = Daemon;
    _ = Nftables;
    _ = Metrics;
}
