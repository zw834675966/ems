# WSL 服务开机自动启动

WSL 本身的 init 系统不包含 systemd（或只在 Windows 11 的最新版本中可选），所以常规 `sudo service … start` 并不会在 Windows 启动时自动执行。不过你可以通过下面两种方式让 PostgreSQL/Redis 在每次 WSL 启动时自动起来。

## 1. 在 WSL 内注册启动脚本（配合 `sudoers` 免密切换）

1. 推荐复用仓库中的辅助脚本：`scripts/start-local-services.sh`。该脚本会尝试通过 `systemctl`/`service` 启动 `postgresql`、`redis-server` 和 `mosquitto`。复制到 `/usr/local/bin/start-db-services.sh` 并授予执行权限即可：
   ```bash
   sudo cp scripts/start-local-services.sh /usr/local/bin/start-db-services.sh
   sudo chmod +x /usr/local/bin/start-db-services.sh
   ```
   （也可以直接在开发机里调用 `bash scripts/start-local-services.sh`，不复制也能用。）

2. 让脚本可以在不输入密码的情况下运行（只对这几个命令）：
   ```bash
   sudo visudo
   ```
   在文件末尾加入（假设当前用户为 `zw`）：
   ```
   zw ALL=(ALL) NOPASSWD: /usr/local/bin/start-db-services.sh, /usr/sbin/service postgresql *, /usr/sbin/service redis-server *
   ```
   这样后续运行脚本时不会再提示密码。

3. 将脚本挂载在 `~/.bashrc` 或 `~/.profile` 中：
   ```bash
   if [[ $(pgrep postgres) == "" ]]; then
     /usr/local/bin/start-db-services.sh
   fi
   ```
   这样每次打开 WSL shell 时都会检查并启动服务。

## 2. 利用 Windows 任务计划在登录时唤醒服务（更稳定）

1. 打开“任务计划程序”，创建一个“登录时触发”的任务。
2. 在“操作”中执行：
   ```text
   Program/script: wsl
   Arguments: -d <你的 distro 名称> -- /bin/bash -lc "sudo /usr/local/bin/start-db-services.sh"
   ```
3. 任务设置为“使用最高权限运行”，并选择合适的用户。
   - 如果你不希望每次都输入密码，可以如前述给 `start-db-services.sh` 或 `service` 命令配置 sudoers 免密。

**提示**：
- 如果使用的是新版 Windows + WSL，`/etc/wsl.conf` 可以启用 systemd：在 `[boot]` 下加 `systemd=true`，然后在 WSL 内 `sudo systemctl enable postgresql` 等服务即可。此方式会在 WSL 每次启动时使用 systemd 管理服务，但依赖 Windows 版本与 distro 支持。
- 任何方案都建议先在 WSL shell 中手动运行脚本确保能成功启动，再让 Windows 或 shell 自动执行。
