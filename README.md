# API-Key-FY

基于 Rust + Vue 3 构建的 API 网关，专为华为云服务设计。通过 API Key 认证代理上游请求，自动完成 IAM Token 的获取与缓存，提供 Web 管理界面进行密钥、应用和日志的管理。

## 功能特性

- **反向代理**：支持基于路径（`http_path`）和基于端口（`http_port`）两种代理模式
- **API Key 管理**：创建、删除、限速、作用域控制；密钥加密存储，支持事后查看
- **IAM 自动鉴权**：自动获取并缓存华为云 IAM Token，注入到上游请求头
- **速率限制**：每个 API Key 独立限速（次/分钟）
- **请求日志**：异步记录代理请求，支持按 Key / App 查询，自动定期清理
- **SSE 支持**：透传服务端推送事件（Server-Sent Events），不缓冲流式响应
- **Web 管理界面**：中文界面，管理凭证、应用、密钥和日志

## 架构

```
客户端
  │  Bearer <api-key>
  ▼
反向代理（Rust / Axum）
  │  1. 验证 API Key（bcrypt）
  │  2. 检查速率限制
  │  3. 获取 IAM Token（缓存）
  │  4. 注入 X-Auth-Token 头
  ▼
上游服务（华为云 API）
```

管理面板（Vue 3 SPA）内嵌在同一个二进制文件中，通过 `/admin/` 路径访问。

## 快速部署

### 前提条件

- Docker + docker-compose
- 华为云 IAM 账号及项目信息
- 镜像仓库访问权限（或使用本地构建）

### 1. 准备配置文件

在部署目录创建 `.env` 文件：

```env
# PostgreSQL 密码（任意强密码）
POSTGRES_PASSWORD=your_strong_password

# AES-256 加密密钥，用于加密存储 IAM 凭证（32字节十六进制）
# 生成命令：openssl rand -hex 32
ENCRYPTION_KEY=your_64_char_hex_key

# 华为云 IAM 接口地址（不含末尾斜杠）
IAM_ENDPOINT=https://iam.myhuaweicloud.com

# 登录页默认域名（可选）
VITE_DEFAULT_DOMAIN=your_domain
```

### 2. 准备 docker-compose 文件

**Docker 20.10+ （`docker compose` 插件）：**

```yaml
# docker-compose.yml
services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: apikeyfy
      POSTGRES_USER: apikeyfy
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U apikeyfy"]
      interval: 5s
      timeout: 5s
      retries: 10

  proxy:
    image: swr.cn-global-1.cloud.nisco.cn/llmspace/api-key-fy:latest
    ports:
      - "8800:8800"
      - "8801-8999:8801-8999"
    environment:
      DATABASE_URL: postgres://apikeyfy:${POSTGRES_PASSWORD}@db/apikeyfy
      ENCRYPTION_KEY: ${ENCRYPTION_KEY}
      IAM_ENDPOINT: ${IAM_ENDPOINT}
      PORT: 8800
      RUST_LOG: info
    depends_on:
      db:
        condition: service_healthy

volumes:
  pgdata:
```

**Docker 18.09 / docker-compose 1.x：** 使用项目中的 `docker-compose.legacy.yml`。

### 3. 启动

**Docker 20.10+：**
```bash
docker compose up -d
```

**Docker 18.09（docker-compose 1.x）：**
```bash
# 注意：sudo 不会自动加载 .env，需手动导入
sudo sh -c 'set -a; . ./.env; set +a; docker-compose -f docker-compose.legacy.yml up -d'
```

### 4. 访问管理界面

浏览器打开 `http://<服务器IP>:8800/`，使用华为云 IAM 账号登录。

---

## 本地开发

### 依赖

- [Rust](https://rustup.rs/) 1.75+
- [Bun](https://bun.sh/) 1.x
- PostgreSQL 14+

### 启动数据库

```bash
docker run -d --name pg \
  -e POSTGRES_DB=apikeyfy \
  -e POSTGRES_USER=apikeyfy \
  -e POSTGRES_PASSWORD=dev \
  -p 5432:5432 \
  postgres:16-alpine
```

### 配置后端

在 `proxy/` 目录创建 `.env`：

```env
DATABASE_URL=postgres://apikeyfy:dev@localhost/apikeyfy
ENCRYPTION_KEY=0000000000000000000000000000000000000000000000000000000000000001
IAM_ENDPOINT=https://iam.myhuaweicloud.com
PORT=8800
RUST_LOG=info
```

### 启动后端

```bash
cd proxy
cargo run
```

首次启动会自动执行数据库迁移。

### 启动前端

```bash
cd admin
bun install
bun run dev   # 开发服务器运行在 :5173，自动代理 /admin/api 到 :8800
```

### 构建镜像

```bash
# 本地架构
docker build -t api-key-fy .

# 指定 x86_64
docker buildx build --platform linux/amd64 -t api-key-fy:latest --load .
```

---

## 使用说明

### 1. 添加凭证

进入「凭证管理」，填写华为云 IAM 账号信息：

| 字段 | 说明 |
|------|------|
| 名称 | 凭证备注名 |
| IAM 用户名 | 华为云子账号用户名 |
| IAM 密码 | 华为云密码（加密存储） |
| Domain | IAM 账号域 |
| Project | 项目名称（如 `cn-north-4`） |
| Region | 地区标识 |
| 自定义 Endpoint | 私有云或代理地址（可选） |

### 2. 创建应用

进入「应用管理」，创建一个代理应用：

| 字段 | 说明 |
|------|------|
| 类型 | `http_path`（路径代理）或 `http_port`（端口代理） |
| Slug | 路径前缀（仅 http_path，如 `v1/infers`） |
| 上游地址 | 目标服务的 Base URL |
| 去除前缀 | 转发时是否去掉 slug 前缀 |
| 监听端口 | 独立监听端口（仅 http_port，范围 8801-8999） |

### 3. 创建 API Key

进入「API 密钥」，点击「新建密钥」：

- 设置名称、速率限制（次/分钟）、过期时间
- 选择该密钥可访问的应用
- 创建后可随时通过「复制密钥」按钮查看完整密钥

### 4. 调用 API

```bash
curl https://<host>:8800/<slug>/your/path \
  -H "Authorization: Bearer akfy_<your_api_key>" \
  -H "Content-Type: application/json" \
  -d '{"key": "value"}'
```

代理会自动验证 API Key、注入 IAM Token 并转发到上游服务。

---

## 环境变量参考

| 变量 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `DATABASE_URL` | ✅ | — | PostgreSQL 连接串 |
| `ENCRYPTION_KEY` | ✅ | — | 64位十六进制 AES-256 密钥 |
| `IAM_ENDPOINT` | ✅ | — | 华为云 IAM 地址 |
| `PORT` | — | `3000` | 主服务监听端口 |
| `LOG_RETENTION_DAYS` | — | `30` | 请求日志保留天数 |
| `RUST_LOG` | — | `info` | 日志级别（debug/info/warn/error） |

构建时变量（Dockerfile ARG）：

| 变量 | 说明 |
|------|------|
| `VITE_DEFAULT_DOMAIN` | 登录页默认填充的 IAM Domain |

---

## 技术栈

| 组件 | 技术 |
|------|------|
| 后端 | Rust / Axum 0.8 / sqlx 0.8 / reqwest 0.12 |
| 前端 | Vue 3 / TypeScript / Pinia / Tailwind CSS v4 |
| 数据库 | PostgreSQL 16 |
| 构建 | Bun（前端）/ Cargo（后端）/ Docker 多阶段构建 |
| 加密 | AES-256-GCM（凭证 & 密钥）/ bcrypt（密钥校验） |
