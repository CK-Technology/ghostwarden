const std = @import("std");
const Allocator = std.mem.Allocator;
const Http = @import("http.zig");
const Config = @import("config.zig");

pub const Alert = struct {
    timestamp: []const u8,
    rule_id: u32,
    rule_level: u8,
    rule_description: []const u8,
    agent_id: []const u8,
    agent_name: []const u8,
    source_ip: []const u8,
    action: AlertAction,
};

pub const AlertAction = enum {
    ban,
    allow,
    monitor,
};

pub const WazuhAlert = struct {
    id: []const u8,
    timestamp: []const u8,
    rule: struct {
        id: u32,
        level: u8,
        description: []const u8,
        groups: [][]const u8,
    },
    agent: struct {
        id: []const u8,
        name: []const u8,
        ip: []const u8,
    },
    data: struct {
        srcip: ?[]const u8 = null,
        dstip: ?[]const u8 = null,
        srcport: ?u16 = null,
        dstport: ?u16 = null,
        protocol: ?[]const u8 = null,
    },
    decoder: struct {
        name: []const u8,
    },
    location: []const u8,
};

pub const WazuhClient = struct {
    allocator: Allocator,
    config: Config.WazuhConfig,
    http_client: Http.HttpClient,
    auth_token: ?[]const u8 = null,
    token_expires: i64 = 0,

    const Self = @This();

    pub fn init(allocator: Allocator, config: Config.WazuhConfig) Self {
        return Self{
            .allocator = allocator,
            .config = config,
            .http_client = Http.HttpClient.init(allocator),
        };
    }

    pub fn deinit(self: *Self) void {
        if (self.auth_token) |token| {
            self.allocator.free(token);
        }
        self.http_client.deinit();
    }

    pub fn authenticate(self: *Self) !void {
        const url = try std.fmt.allocPrint(self.allocator, "{s}/security/user/authenticate", .{self.config.api_url});
        defer self.allocator.free(url);

        const auth = Http.HttpClient.AuthHeader{ .basic = .{ .username = self.config.api_user, .password = self.config.api_pass } };
        var response = self.http_client.post(url, "", auth) catch |err| switch (err) {
            error.NetworkError => {
                std.log.err("Failed to connect to Wazuh API at {s}", .{self.config.api_url});
                return error.NetworkError;
            },
            else => return err,
        };
        defer response.deinit();

        switch (response.status) {
            .ok => {},
            .unauthorized => {
                std.log.err("Wazuh API authentication failed. Check your credentials.", .{});
                return error.AuthenticationError;
            },
            .not_found => {
                std.log.err("Wazuh API endpoint not found. Check your API URL.", .{});
                return error.ApiError;
            },
            else => {
                std.log.err("Wazuh API returned status: {}", .{response.status});
                return error.ApiError;
            },
        }

        const token = try self.parseAuthResponse(response.body);
        if (self.auth_token) |old_token| {
            self.allocator.free(old_token);
        }
        self.auth_token = token;
        self.token_expires = std.time.timestamp() + 3600; // Token valid for 1 hour
    }

    fn parseAuthResponse(self: *Self, body: []const u8) ![]const u8 {
        const parsed = std.json.parseFromSlice(std.json.Value, self.allocator, body, .{}) catch |err| switch (err) {
            error.SyntaxError => {
                std.log.err("Invalid JSON response from Wazuh API", .{});
                return error.ParseError;
            },
            else => return err,
        };
        defer parsed.deinit();

        const root = parsed.value;
        if (root.object.get("data")) |data| {
            if (data.object.get("token")) |token| {
                return try self.allocator.dupe(u8, token.string);
            }
        }

        return error.ParseError;
    }

    fn ensureAuthenticated(self: *Self) !void {
        const now = std.time.timestamp();
        if (self.auth_token == null or now >= self.token_expires - 300) { // Refresh 5 minutes before expiry
            try self.authenticate();
        }
    }

    pub fn getAlerts(self: *Self, from_timestamp: ?[]const u8, limit: u32) ![]WazuhAlert {
        try self.ensureAuthenticated();

        var url_builder = std.ArrayList(u8).init(self.allocator);
        defer url_builder.deinit();

        try url_builder.writer().print("{s}/alerts?pretty=true&limit={d}", .{ self.config.api_url, limit });

        if (from_timestamp) |timestamp| {
            try url_builder.writer().print("&timestamp>={s}", .{timestamp});
        }

        const url = try url_builder.toOwnedSlice();
        defer self.allocator.free(url);

        const auth = Http.HttpClient.AuthHeader{ .bearer = self.auth_token.? };
        var response = self.http_client.get(url, auth) catch |err| switch (err) {
            error.NetworkError => {
                std.log.err("Failed to connect to Wazuh API", .{});
                return error.NetworkError;
            },
            else => return err,
        };
        defer response.deinit();

        switch (response.status) {
            .ok => {},
            .unauthorized => {
                self.auth_token = null;
                try self.authenticate();
                return self.getAlerts(from_timestamp, limit);
            },
            else => {
                std.log.err("Wazuh API returned status: {}", .{response.status});
                return error.ApiError;
            },
        }

        return self.parseAlerts(response.body);
    }

    fn parseAlerts(self: *Self, body: []const u8) ![]WazuhAlert {
        const parsed = std.json.parseFromSlice(std.json.Value, self.allocator, body, .{}) catch |err| switch (err) {
            error.SyntaxError => {
                std.log.err("Invalid JSON response from Wazuh API", .{});
                return error.ParseError;
            },
            else => return err,
        };
        defer parsed.deinit();

        const root = parsed.value;
        if (root.object.get("data")) |data| {
            if (data.object.get("affected_items")) |items| {
                if (items == .array) {
                    return self.parseAlertArray(items.array.items);
                }
            }
        }

        return error.ParseError;
    }

    fn parseAlertArray(self: *Self, alerts: []std.json.Value) ![]WazuhAlert {
        var result = try std.ArrayList(WazuhAlert).initCapacity(self.allocator, alerts.len);
        defer result.deinit();

        for (alerts) |alert_value| {
            if (alert_value != .object) continue;
            const obj = alert_value.object;

            const alert = WazuhAlert{
                .id = if (obj.get("id")) |v| try self.allocator.dupe(u8, v.string) else "",
                .timestamp = if (obj.get("timestamp")) |v| try self.allocator.dupe(u8, v.string) else "",
                .rule = .{
                    .id = if (obj.get("rule")) |rule| blk: {
                        if (rule.object.get("id")) |id| break :blk @intCast(id.integer);
                        break :blk 0;
                    } else 0,
                    .level = if (obj.get("rule")) |rule| blk: {
                        if (rule.object.get("level")) |level| break :blk @intCast(level.integer);
                        break :blk 0;
                    } else 0,
                    .description = if (obj.get("rule")) |rule| blk: {
                        if (rule.object.get("description")) |desc| break :blk try self.allocator.dupe(u8, desc.string);
                        break :blk "";
                    } else "",
                    .groups = &.{},
                },
                .agent = .{
                    .id = if (obj.get("agent")) |agent| blk: {
                        if (agent.object.get("id")) |id| break :blk try self.allocator.dupe(u8, id.string);
                        break :blk "";
                    } else "",
                    .name = if (obj.get("agent")) |agent| blk: {
                        if (agent.object.get("name")) |name| break :blk try self.allocator.dupe(u8, name.string);
                        break :blk "";
                    } else "",
                    .ip = if (obj.get("agent")) |agent| blk: {
                        if (agent.object.get("ip")) |ip| break :blk try self.allocator.dupe(u8, ip.string);
                        break :blk "";
                    } else "",
                },
                .data = .{
                    .srcip = if (obj.get("data")) |data| blk: {
                        if (data.object.get("srcip")) |ip| break :blk try self.allocator.dupe(u8, ip.string);
                        break :blk null;
                    } else null,
                },
                .decoder = .{
                    .name = if (obj.get("decoder")) |decoder| blk: {
                        if (decoder.object.get("name")) |name| break :blk try self.allocator.dupe(u8, name.string);
                        break :blk "";
                    } else "",
                },
                .location = if (obj.get("location")) |v| try self.allocator.dupe(u8, v.string) else "",
            };

            try result.append(alert);
        }

        return result.toOwnedSlice();
    }

    pub fn convertToActions(self: *Self, alerts: []WazuhAlert) ![]Alert {
        var result = try std.ArrayList(Alert).initCapacity(self.allocator, alerts.len);
        defer result.deinit();

        for (alerts) |wazuh_alert| {
            if (wazuh_alert.data.srcip) |srcip| {
                const action = self.determineAction(wazuh_alert.rule.level, wazuh_alert.rule.id);
                if (action == .monitor) continue;

                const alert = Alert{
                    .timestamp = try self.allocator.dupe(u8, wazuh_alert.timestamp),
                    .rule_id = wazuh_alert.rule.id,
                    .rule_level = wazuh_alert.rule.level,
                    .rule_description = try self.allocator.dupe(u8, wazuh_alert.rule.description),
                    .agent_id = try self.allocator.dupe(u8, wazuh_alert.agent.id),
                    .agent_name = try self.allocator.dupe(u8, wazuh_alert.agent.name),
                    .source_ip = try self.allocator.dupe(u8, srcip),
                    .action = action,
                };

                try result.append(alert);
            }
        }

        return result.toOwnedSlice();
    }

    fn determineAction(self: *Self, level: u8, rule_id: u32) AlertAction {
        _ = self;
        _ = rule_id;

        return switch (level) {
            0...5 => .monitor,
            6...10 => .allow,
            11...15 => .ban,
            else => .ban,
        };
    }

    pub fn validateConfig(config: Config.WazuhConfig) !void {
        if (config.api_url.len == 0) {
            return error.ConfigurationError;
        }
        if (config.api_user.len == 0) {
            return error.ConfigurationError;
        }
        if (config.api_pass.len == 0) {
            return error.ConfigurationError;
        }
    }
};

test "wazuh client init" {
    const config = Config.WazuhConfig{
        .api_url = "https://wazuh.example.com",
        .api_user = "test-user",
        .api_pass = "test-pass",
    };

    var client = WazuhClient.init(std.testing.allocator, config);
    defer client.deinit();
}