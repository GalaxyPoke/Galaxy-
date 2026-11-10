# 启动器数据接口（GalaxyLauncherApi）

启动器首页的服务器状态、在线玩家头像、玩家数据卡、活动倒计时、实时公告和「一键上传崩溃报告」都走这个接口。接口由正式服上的 Bukkit 插件 **GalaxyLauncherApi** 提供（源码 `D:\GalaxyPokemon\_devtools\galaxylauncherapi`），启动器只通过后端命令访问它（webview 的 CSP 不放行这个地址）。

## 地址

- 插件监听 `0.0.0.0:8095`（`plugins/GalaxyLauncherApi/config.yml` 的 `http.bind` / `http.port`）。
- 公网地址由服主在服务商那里做端口映射，默认约定 **公网 28095 → 本机 8095**，启动器里写成 `http://mc9.yunmaoc.com:28095`。
- 启动器从 `launcher\galaxy-launcher.json` 的 `apiBase` 读地址（`galaxy-stage.ps1 -ApiBase` 写入，默认上面的地址）；开发时可用环境变量 `GALAXY_API_BASE`。两处都没有时，这些功能直接隐藏。
- 接口连不上时启动器照常工作：服务器状态退回到 Velocity 的 ping，公告退回到缓存 / 内置列表，玩家卡和活动区隐藏，崩溃上传按钮提示「服务器暂时收不到」。

## 通用约定

- 只用 HTTP/1.1 + JSON，UTF-8。响应头 `Content-Type: application/json; charset=utf-8`、`Cache-Control: no-store`。
- 所有响应都带 `"schema": 1`。字段只增不改；启动器忽略不认识的字段。
- 时间一律是服务器本地时间（Asia/Shanghai）的 `YYYY-MM-DDTHH:mm` 或 `YYYY-MM-DDTHH:mm:ss`，不带时区，和公告的 `created-at` 一样。
- 出错时返回对应状态码和 `{"schema":1,"ok":false,"error":"中文说明"}`：400 参数不对、404 路径不存在、405 方法不对、413 内容太大、429 太频繁、503 服务器正在启动 / 关闭。
- 限流按来源 IP：GET 每分钟 60 次，POST /v1/crash 每小时 5 次、每天全服 200 次。

## GET /v1/health

```json
{ "schema": 1, "ok": true, "version": "1.0.0" }
```

## GET /v1/summary

每 5 秒在主线程生成一次快照，HTTP 线程直接返回快照。

```json
{
  "schema": 1,
  "generatedAt": "2026-11-05T20:15:30",
  "server": {
    "name": "Galaxy宝可梦",
    "online": 3,
    "max": 20,
    "tps": 19.9,
    "players": [ { "name": "Steve", "uuid": "8667ba71-b85a-4004-af54-457a9734eed7" } ],
    "maintenance": { "active": false, "message": "" }
  },
  "events": [
    {
      "id": "battlepass-s1",
      "kind": "season",
      "title": "S1 通行证赛季",
      "description": "赛季结束后通行证等级和通行证币清零",
      "start": "2026-11-02T00:00",
      "end": "2026-11-30T00:00"
    }
  ]
}
```

- `players`：最多 50 个，按名字排序；隐身的玩家（带 `vanished` 元数据）不列出，也不计入 `online`。
- `tps`：插件自己按最近 100 tick 的间隔算，保留 1 位小数，最高 20.0。
- `maintenance.active`：`config.yml` 的 `maintenance.active`，或者服务器开了白名单。`/galaxyapi maintenance on <说明>` / `off` 可以随时切换。
- `events`：`config.yml` 的 `events` 列表（服主手写），加上 BattlePass 当前赛季（能从 BattlePass 配置读出起止时间时）。已经结束超过 1 天的不返回。`kind` 取 `season` / `activity` / `maintenance`。

## GET /v1/announcements

把 `plugins/PlayerAnnouncement/announcements.yml` 原样转成 JSON（文件改了就重新读）。字段和启动器的 `GalaxyAnnouncement` 一致，启动器的解析器同时认 `created-at` / `createdAt` 两种写法。`enabled: false` 的不返回。

```json
{
  "schema": 1,
  "announcements": [
    {
      "id": "1",
      "title": "&b欢迎来到银河宝可梦",
      "content": ["&f欢迎各位训练家！"],
      "category": "服务器公告",
      "pinned": true,
      "mustRead": true,
      "priority": 100,
      "createdAt": "2026-10-30T18:00",
      "startAt": null,
      "endAt": null,
      "button": { "text": "&a[打开活动中心]", "action": "RUN_COMMAND", "value": "/activity" }
    }
  ]
}
```

## GET /v1/player/{name}

`name` 必须匹配 `^[A-Za-z0-9_]{3,16}$`，否则 400。结果按名字缓存 30 秒。数据在主线程读（`callSyncMethod`，最多等 3 秒）。

```json
{
  "schema": 1,
  "name": "Steve",
  "known": true,
  "online": false,
  "firstJoined": "2026-11-01T12:00:00",
  "lastSeen": "2026-11-05T19:40:12",
  "money": 12345.5,
  "moneyName": "金币",
  "points": 30,
  "level": 12,
  "pass": { "tier": 15, "maxTier": 120, "season": "S1", "seasonEnd": "2026-11-30T00:00" },
  "signedInToday": false,
  "signInStreak": 3,
  "reminders": [ { "id": "signin", "text": "今天还没签到", "command": "/signin" } ]
}
```

- 没进过正式服的名字：`{"schema":1,"name":"xxx","known":false}`（状态码 200）。
- 读不到的字段返回 `null`（比如对应插件没装、占位符对离线玩家不生效）。启动器只显示有值的格子。
- 数据来源：金币走 Vault（XConomy），点券、等级、通行证、签到走 PlaceholderAPI，占位符写在 `config.yml` 的 `player.placeholders`，服主可以改。
- `config.yml` 的 `player.enabled: false` 可以整段关掉这个接口（返回 404）。

## POST /v1/crash

请求体最大 2 MB（`Content-Length` 超了直接 413）。

```json
{
  "schema": 1,
  "name": "Steve",
  "launcherVersion": "1.0.7",
  "modpackVersion": "1.0.5",
  "os": "Windows 11 (10.0.22631) x86_64",
  "javaVersion": "21.0.12",
  "memoryMb": 6144,
  "summary": "游戏启动后崩溃（退出码 -1）",
  "files": [
    { "name": "crash-2026-11-05_20.10.11-client.txt", "content": "..." },
    { "name": "latest.log", "content": "...（只取最后 1 MB）" }
  ]
}
```

- `name` 规则同上，可以为空。`files` 最多 4 个，`name` 只保留 `[A-Za-z0-9._-]`。
- 保存到 `plugins/GalaxyLauncherApi/crashes/<yyyy-MM-dd>/<id>_<name>.txt`（一个文本文件，开头是元数据，后面按文件分段）。目录总大小超过 `crash.max-total-mb`（默认 500）时删最旧的。
- 返回 `{"schema":1,"ok":true,"id":"20261105-201530-7f3a"}`。在线的管理员（`galaxyapi.admin`）会收到一条聊天提示。
- `/galaxyapi crashes [数量]` 列出最近的上传。

## 管理命令

`/galaxyapi status | reload | maintenance on <说明> | maintenance off | crashes [数量]`，权限 `galaxyapi.admin`（默认 OP）。
