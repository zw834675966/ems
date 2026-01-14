# EMS_JWT_SECRET 配置（本地可运行）

已为本地运行生成 `.env`，`cargo run -p ems-api` 会自动加载该文件。

当前 `.env` 内容：
```bash
EMS_JWT_SECRET="c348212b6b4a0f29032cd1deb5718bc45f6ca2f1cb368279719792180fb41295"
EMS_JWT_ACCESS_TTL_SECONDS="3600"
EMS_JWT_REFRESH_TTL_SECONDS="2592000"
EMS_HTTP_ADDR="127.0.0.1:8080"
EMS_DATABASE_URL="postgresql://ems:admin123@localhost:5432/ems"
```

说明：
- `EMS_JWT_SECRET` 使用 32 字节随机值生成（hex 字符串）。
- `EMS_JWT_ACCESS_TTL_SECONDS` 设置 access token 有效期为 1 小时。
- `EMS_JWT_REFRESH_TTL_SECONDS` 设置 refresh token 有效期为 30 天。
- `EMS_DATABASE_URL` 使用本地 Postgres 连接串。
