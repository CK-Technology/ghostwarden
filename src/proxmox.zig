const std = @import("std");
const Allocator = std.mem.Allocator;
const Http = @import("http.zig");
const Config = @import("config.zig");

pub const IpSetEntry = struct {
    cidr: []const u8,
    nomatch: bool = false,
    comment: ?[]const u8 = null,
};

pub const IpSet = struct {
    name: []const u8,
    entries: []IpSetEntry,
};

pub const ProxmoxClient = struct {
    allocator: Allocator,
    config: Config.ProxmoxConfig,
    http_client: Http.HttpClient,
    csrf_token: ?[]const u8 = null,
    ticket: ?[]const u8 = null,

    const Self = @This();

    pub fn init(allocator: Allocator, config: Config.ProxmoxConfig) Self {
        return Self{
            .allocator = allocator,
            .config = config,
            .http_client = Http.HttpClient.init(allocator),
        };
    }

    pub fn deinit(self: *Self) void {
        if (self.csrf_token) |token| {
            self.allocator.free(token);
        }
        if (self.ticket) |ticket| {
            self.allocator.free(ticket);
        }
        self.http_client.deinit();
    }

    fn getAuthHeader(self: *Self) Http.HttpClient.AuthHeader {
        return Http.HttpClient.AuthHeader{ .pve_token = .{
            .token_id = self.config.token_id,
            .token_secret = self.config.token_secret,
        } };
    }

    pub fn getIpSet(self: *Self, ipset_name: []const u8) !IpSet {
        const url = try std.fmt.allocPrint(self.allocator, "{s}/cluster/firewall/ipset/{s}", .{ self.config.api_url, ipset_name });
        defer self.allocator.free(url);

        const auth = self.getAuthHeader();
        var response = self.http_client.get(url, auth) catch |err| {
            std.log.err("Failed to connect to Proxmox VE API at {s}: {}", .{ self.config.api_url, err });
            return err;
        };
        defer response.deinit();

        switch (response.status) {
            .ok => {},
            .unauthorized => {
                std.log.err("Proxmox VE API authentication failed. Check your token.", .{});
                return error.AuthenticationError;
            },
            .not_found => {
                return self.createIpSet(ipset_name);
            },
            else => {
                std.log.err("Proxmox VE API returned status: {}", .{response.status});
                return error.ApiError;
            },
        }

        return self.parseIpSetResponse(response.body, ipset_name);
    }

    fn parseIpSetResponse(self: *Self, body: []const u8, name: []const u8) !IpSet {
        const parsed = std.json.parseFromSlice(std.json.Value, self.allocator, body, .{}) catch |err| switch (err) {
            error.SyntaxError => {
                std.log.err("Invalid JSON response from Proxmox VE API", .{});
                return error.ParseError;
            },
            else => return err,
        };
        defer parsed.deinit();

        const root = parsed.value;
        if (root.object.get("data")) |data| {
            if (data == .array) {
                return IpSet{
                    .name = try self.allocator.dupe(u8, name),
                    .entries = try self.parseIpSetEntries(data.array.items),
                };
            }
        }

        return IpSet{
            .name = try self.allocator.dupe(u8, name),
            .entries = &.{},
        };
    }

    fn parseIpSetEntries(self: *Self, entries: []std.json.Value) ![]IpSetEntry {
        var result = try std.ArrayList(IpSetEntry).initCapacity(self.allocator, entries.len);
        defer result.deinit();

        for (entries) |entry_value| {
            if (entry_value != .object) continue;
            const obj = entry_value.object;

            const entry = IpSetEntry{
                .cidr = if (obj.get("cidr")) |v| try self.allocator.dupe(u8, v.string) else "",
                .nomatch = if (obj.get("nomatch")) |v| v.bool else false,
                .comment = if (obj.get("comment")) |v| try self.allocator.dupe(u8, v.string) else null,
            };

            try result.append(entry);
        }

        return result.toOwnedSlice(self.allocator);
    }

    pub fn createIpSet(self: *Self, ipset_name: []const u8) !IpSet {
        const url = try std.fmt.allocPrint(self.allocator, "{s}/cluster/firewall/ipset", .{self.config.api_url});
        defer self.allocator.free(url);

        const body = try std.fmt.allocPrint(self.allocator, "name={s}&comment=Ghostwarden%20managed%20IPSet", .{ipset_name});
        defer self.allocator.free(body);

        var headers = std.StringHashMap([]const u8).init(self.allocator);
        defer headers.deinit();
        try headers.put("content-type", "application/x-www-form-urlencoded");

        const auth = self.getAuthHeader();
        var response = self.http_client.request(.{
            .method = .POST,
            .url = url,
            .body = body,
            .auth = auth,
            .headers = headers,
        }) catch |err| {
            std.log.err("Failed to create IPSet in Proxmox VE: {}", .{err});
            return err;
        };
        defer response.deinit();

        switch (response.status) {
            .ok => {
                std.log.info("Created IPSet '{s}' in Proxmox VE cluster", .{ipset_name});
                return IpSet{
                    .name = try self.allocator.dupe(u8, ipset_name),
                    .entries = &.{},
                };
            },
            .unauthorized => {
                std.log.err("Proxmox VE API authentication failed during IPSet creation", .{});
                return error.AuthenticationError;
            },
            else => {
                std.log.err("Failed to create IPSet. Proxmox VE API returned status: {}", .{response.status});
                return error.ApiError;
            },
        }
    }

    pub fn addIpToSet(self: *Self, ipset_name: []const u8, ip: []const u8, comment: ?[]const u8) !void {
        const url = try std.fmt.allocPrint(self.allocator, "{s}/cluster/firewall/ipset/{s}", .{ self.config.api_url, ipset_name });
        defer self.allocator.free(url);

        const body = if (comment) |c|
            try std.fmt.allocPrint(self.allocator, "cidr={s}&comment={s}", .{ ip, c })
        else
            try std.fmt.allocPrint(self.allocator, "cidr={s}", .{ip});
        defer self.allocator.free(body);

        var headers = std.StringHashMap([]const u8).init(self.allocator);
        defer headers.deinit();
        try headers.put("content-type", "application/x-www-form-urlencoded");

        const auth = self.getAuthHeader();
        var response = self.http_client.request(.{
            .method = .POST,
            .url = url,
            .body = body,
            .auth = auth,
            .headers = headers,
        }) catch |err| {
            std.log.err("Failed to add IP {s} to IPSet {s}: {}", .{ ip, ipset_name, err });
            return err;
        };
        defer response.deinit();

        switch (response.status) {
            .ok => {
                std.log.debug("Added IP {s} to IPSet {s}", .{ ip, ipset_name });
            },
            .unauthorized => {
                std.log.err("Proxmox VE API authentication failed", .{});
                return error.AuthenticationError;
            },
            .unprocessable_entity => {
                std.log.warn("IP {s} already exists in IPSet {s}", .{ ip, ipset_name });
            },
            else => {
                std.log.err("Failed to add IP {s} to IPSet {s}. Status: {}", .{ ip, ipset_name, response.status });
                return error.ApiError;
            },
        }
    }

    pub fn removeIpFromSet(self: *Self, ipset_name: []const u8, ip: []const u8) !void {
        const encoded_ip = try self.urlEncode(ip);
        defer self.allocator.free(encoded_ip);

        const url = try std.fmt.allocPrint(self.allocator, "{s}/cluster/firewall/ipset/{s}/{s}", .{ self.config.api_url, ipset_name, encoded_ip });
        defer self.allocator.free(url);

        const auth = self.getAuthHeader();
        var response = self.http_client.delete(url, auth) catch |err| {
            std.log.err("Failed to remove IP {s} from IPSet {s}: {}", .{ ip, ipset_name, err });
            return err;
        };
        defer response.deinit();

        switch (response.status) {
            .ok => {
                std.log.debug("Removed IP {s} from IPSet {s}", .{ ip, ipset_name });
            },
            .unauthorized => {
                std.log.err("Proxmox VE API authentication failed", .{});
                return error.AuthenticationError;
            },
            .not_found => {
                std.log.warn("IP {s} not found in IPSet {s}", .{ ip, ipset_name });
            },
            else => {
                std.log.err("Failed to remove IP {s} from IPSet {s}. Status: {}", .{ ip, ipset_name, response.status });
                return error.ApiError;
            },
        }
    }

    pub fn bulkUpdateIpSet(self: *Self, ipset_name: []const u8, ips_to_add: []const []const u8, ips_to_remove: []const []const u8) !void {
        for (ips_to_remove) |ip| {
            self.removeIpFromSet(ipset_name, ip) catch |err| {
                std.log.warn("Failed to remove IP {s}: {}", .{ ip, err });
            };
        }

        for (ips_to_add) |ip| {
            const comment = try std.fmt.allocPrint(self.allocator, "Ghostwarden ban - {d}", .{std.time.timestamp()});
            defer self.allocator.free(comment);

            self.addIpToSet(ipset_name, ip, comment) catch |err| {
                std.log.warn("Failed to add IP {s}: {}", .{ ip, err });
            };
        }

        std.log.info("Updated IPSet {s}: added {d}, removed {d} IPs", .{ ipset_name, ips_to_add.len, ips_to_remove.len });
    }

    pub fn testConnection(self: *Self) !void {
        const url = try std.fmt.allocPrint(self.allocator, "{s}/version", .{self.config.api_url});
        defer self.allocator.free(url);

        const auth = self.getAuthHeader();
        var response = self.http_client.get(url, auth) catch |err| {
            std.log.err("Failed to connect to Proxmox VE API at {s}: {}", .{ self.config.api_url, err });
            return err;
        };
        defer response.deinit();

        switch (response.status) {
            .ok => {
                std.log.info("Successfully connected to Proxmox VE API", .{});
            },
            .unauthorized => {
                std.log.err("Proxmox VE API authentication failed. Check your token.", .{});
                return error.AuthenticationError;
            },
            else => {
                std.log.err("Proxmox VE API connection test failed. Status: {}", .{response.status});
                return error.ApiError;
            },
        }
    }

    fn urlEncode(self: *Self, input: []const u8) ![]const u8 {
        var result: std.ArrayList(u8) = .empty;
        defer result.deinit(self.allocator);

        for (input) |c| {
            switch (c) {
                '/' => try result.appendSlice(self.allocator, "%2F"),
                ':' => try result.appendSlice(self.allocator, "%3A"),
                ' ' => try result.appendSlice(self.allocator, "%20"),
                else => try result.append(self.allocator, c),
            }
        }

        return result.toOwnedSlice(self.allocator);
    }

    pub fn validateConfig(config: Config.ProxmoxConfig) !void {
        if (config.api_url.len == 0) {
            return error.ConfigurationError;
        }
        if (config.token_id.len == 0) {
            return error.ConfigurationError;
        }
        if (config.token_secret.len == 0) {
            return error.ConfigurationError;
        }
    }
};

test "proxmox client init" {
    const config = Config.ProxmoxConfig{
        .api_url = "https://pve.example.com:8006/api2/json",
        .token_id = "root@pam!ghostwarden",
        .token_secret = "test-secret",
    };

    var client = ProxmoxClient.init(std.testing.allocator, config);
    defer client.deinit();
}