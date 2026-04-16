---
title: "网页搜索"
description: "让您的智能体使用 Firecrawl 搜索网页"
icon: globe
---

网页搜索工具允许您的智能体使用自托管的 [Firecrawl](https://firecrawl.dev) 实例搜索网页和抓取内容。它从网页返回干净的 markdown，处理 JavaScript 渲染的内容，并且本地实例不需要 API 密钥。

该工具支持两种模式：
- **search**：网页搜索，带有完整页面内容提取
- **context**：抓取特定 URL 用于 RAG grounding

---

## 设置

<Steps>

<Step title="启动 Firecrawl 实例">

### 本地（推荐）

Firecrawl 可以免费自托管。如果您有 IronClaw 仓库的检出：

```bash
cd externals/firecrawl/apps/api
pnpm install
pnpm run start
```

API 将在 `http://localhost:3002` 可用。

### Docker

```bash
docker run -d -p 3002:3002 --name firecrawl firecrawl/firecrawl
```

### 云 API

使用托管的 Firecrawl API，访问 [firecrawl.dev](https://firecrawl.dev)。您需要一个 API 密钥。

</Step>

<Step title="安装网页搜索扩展">

该扩展包含在默认注册表中。使用以下命令安装：

```bash
ironclaw registry install web_search
```

</Step>

<Step title="配置（本地）">

对于本地实例，不需要 API 密钥。该工具会自动连接到 `http://localhost:3002`。

如果您的 Firecrawl 实例运行在不同的 URL 上，设置环境变量：

```bash
export FIRECRAWL_API_URL=http://your-host:3002
```

</Step>

<Step title="配置（云端）">

如果使用云 API，配置您的 API 密钥：

```bash
ironclaw tool auth web_search
```

或设置环境变量：

```bash
export FIRECRAWL_API_KEY=fc-YOUR_API_KEY
export FIRECRAWL_API_URL=https://api.firecrawl.dev
```

</Step>

</Steps>

---

## 使用

### 网页搜索

```json
{
  "query": "rust 编程语言",
  "mode": "search",
  "count": 5
}
```

### RAG Grounding

```json
{
  "query": "总结此页面",
  "mode": "context",
  "url": "https://docs.rs"
}
```
