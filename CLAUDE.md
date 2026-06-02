# “观复”——文献论争图谱工具开发指令

你将作为资深全栈工程师，使用 **Tauri + Vue 3 + Vite + TypeScript + Rust** 构建一款桌面端科研辅助应用。应用名为 **“观复”**。

请严格遵循本文档进行开发，保证代码高内聚、低耦合、类型安全、模块清晰、可维护。MVP 阶段 AI 调用可以使用模拟数据，但必须预留真实接口切换能力。

---

## 1. 项目定位

“观复”是一款面向社科研究者的桌面端科研辅助应用，帮助用户将一批孤立的论文 PDF 转化为一张可视化的学术论争地图。

核心反馈环：

```text
导入论文
→ 自动 / 手动提取结构化主张
→ 建立文献间支持、反对、修正、继承、再诠释关系
→ 交互式图谱可视化
→ 发现研究空白与机会点
```

---

## 2. 技术栈

### 2.1 前端

- Vue 3
- Composition API
- `<script setup>`
- Vite
- TypeScript
- Pinia
- Vue Router，仅用于视图切换
- SCSS Modules
- Cytoscape.js
- pdfjs-dist
- Lucide Icons

### 2.2 桌面端 / 后端

- Tauri 2
- Rust
- Tauri Commands
- Tauri State 管理应用级服务
- `serde` / `serde_json`
- `tokio`
- `reqwest`
- `uuid`
- `chrono`
- `thiserror` / `anyhow`
- 文件系统访问通过 Tauri 后端完成

### 2.3 核心原则

- 不使用 Electron。
- 不依赖 UI 框架，所有界面使用手写 SCSS。
- API Key、文件系统、网络请求、AI 请求、PDF 文本抽取、元数据抓取等敏感或系统级能力必须放在 Tauri Rust 后端。
- 前端只能通过 `@tauri-apps/api/core` 的 `invoke()` 调用后端命令。
- 所有数据结构前后端必须保持类型一致。
- 后端 Rust 类型与前端 TypeScript 类型需要显式维护映射。
- MVP 阶段允许对复杂能力使用 mock，但必须保留真实实现接口。

---

## 3. 全局设计规范

整体风格为现代黑白学术工具风格，冷静、克制、清晰。

### 3.1 色彩

```scss
--color-bg: #FFFFFF;
--color-panel: #FAFAFA;

--color-text-primary: #1A1A1A;
--color-text-secondary: #555555;
--color-text-disabled: #999999;

--color-border: #E0E0E0;

--color-relation-supports: #2C2C2C;
--color-relation-opposes: #1A1A1A;
--color-relation-modifies: #4D4D4D;
--color-relation-adopts: #808080;
--color-relation-reinterprets: #B3B3B3;

--color-node-bg: #EAEAEA;
--color-node-border: #333333;
--color-node-selected-border: #000000;
```

### 3.2 字体

- 默认使用 HarmonyOS Sans。
- fallback 使用系统字体：
  - macOS: `-apple-system`
  - Windows: `Segoe UI`
  - Linux: `system-ui`

示例：

```scss
font-family: "HarmonyOS Sans", -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
```

### 3.3 间距与圆角

- 卡片内边距：`16px`
- 元素间距：以 `8px` / `12px` 为基准
- 输入框、按钮圆角：`2px`
- 卡片几乎无圆角或小圆角
- 阴影极轻：

```scss
box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
```

### 3.4 布局

采用经典三栏式：

```text
左侧导航栏：48px，展开后 240px，仅图标或图标 + 文字
中间主工作区：自适应
右侧属性面板：320px，无选中项时可收起
```

---

## 4. 推荐项目结构

请优先按以下结构组织代码：

```text
guanfu/
├── CLAUDE.md
├── package.json
├── vite.config.ts
├── tsconfig.json
├── index.html
├── src/
│   ├── main.ts
│   ├── App.vue
│   ├── router/
│   │   └── index.ts
│   ├── stores/
│   │   ├── projectStore.ts
│   │   ├── paperStore.ts
│   │   ├── graphStore.ts
│   │   ├── settingsStore.ts
│   │   └── insightStore.ts
│   ├── types/
│   │   ├── project.ts
│   │   ├── paper.ts
│   │   ├── relation.ts
│   │   ├── annotation.ts
│   │   ├── settings.ts
│   │   └── tauri.ts
│   ├── api/
│   │   ├── tauriClient.ts
│   │   ├── projectApi.ts
│   │   ├── paperApi.ts
│   │   ├── metadataApi.ts
│   │   ├── aiApi.ts
│   │   └── graphApi.ts
│   ├── components/
│   │   ├── layout/
│   │   │   ├── AppShell.vue
│   │   │   ├── Sidebar.vue
│   │   │   ├── MainWorkspace.vue
│   │   │   └── RightPanel.vue
│   │   ├── papers/
│   │   │   ├── PaperList.vue
│   │   │   ├── PaperImportDropzone.vue
│   │   │   ├── PaperDetail.vue
│   │   │   ├── MetadataEditor.vue
│   │   │   └── RelationList.vue
│   │   ├── pdf/
│   │   │   ├── PdfReader.vue
│   │   │   ├── PdfPage.vue
│   │   │   └── AnnotationPopup.vue
│   │   ├── graph/
│   │   │   ├── GraphCanvas.vue
│   │   │   ├── GraphToolbar.vue
│   │   │   ├── RelationEditor.vue
│   │   │   ├── ControversyOverlay.vue
│   │   │   └── HypothesisPanel.vue
│   │   ├── insights/
│   │   │   ├── InsightPanel.vue
│   │   │   └── InsightCard.vue
│   │   ├── settings/
│   │   │   ├── SettingsPanel.vue
│   │   │   └── AiProviderForm.vue
│   │   └── common/
│   │       ├── IconButton.vue
│   │       ├── TextInput.vue
│   │       ├── SelectInput.vue
│   │       ├── Modal.vue
│   │       └── EmptyState.vue
│   ├── composables/
│   │   ├── useProject.ts
│   │   ├── usePapers.ts
│   │   ├── usePdfSelection.ts
│   │   ├── useGraph.ts
│   │   ├── useInsights.ts
│   │   └── useKeyboardShortcuts.ts
│   ├── styles/
│   │   ├── globals.scss
│   │   ├── variables.scss
│   │   └── mixins.scss
│   └── utils/
│       ├── ids.ts
│       ├── date.ts
│       ├── relationStyle.ts
│       └── graphTransform.ts
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── project_commands.rs
│       │   ├── paper_commands.rs
│       │   ├── metadata_commands.rs
│       │   ├── ai_commands.rs
│       │   ├── graph_commands.rs
│       │   └── settings_commands.rs
│       ├── services/
│       │   ├── mod.rs
│       │   ├── project_service.rs
│       │   ├── paper_service.rs
│       │   ├── metadata_resolver.rs
│       │   ├── ai_manager.rs
│       │   ├── pdf_text_extractor.rs
│       │   ├── cnki_resolver.rs
│       │   ├── crossref_client.rs
│       │   ├── arxiv_client.rs
│       │   ├── openalex_client.rs
│       │   ├── insight_engine.rs
│       │   └── graph_service.rs
│       ├── models/
│       │   ├── mod.rs
│       │   ├── project.rs
│       │   ├── paper.rs
│       │   ├── relation.rs
│       │   ├── annotation.rs
│       │   ├── settings.rs
│       │   └── insight.rs
│       ├── errors/
│       │   └── mod.rs
│       ├── prompts/
│       │   ├── literature_parse.md
│       │   ├── relation_recommendation.md
│       │   └── insight_generation.md
│       └── state.rs
```

---

## 5. 数据模型

前端 TypeScript 与后端 Rust 必须保持字段一致。

### 5.1 TypeScript 类型

```ts
export interface Project {
  id: string;
  name: string;
  path: string;
  papers: Paper[];
  relations: Relation[];
  annotations: Annotation[];
  hypotheses: HypothesisNode[];
  graphLayout: GraphLayout;
  settings?: ProjectSettings;
  createdAt: string;
  updatedAt: string;
}

export interface Paper {
  id: string;
  title: string;
  authors: string[];
  year?: number;
  abstract?: string;
  filePath: string;
  metadata?: ExtractedMetadata;
  tags: string[];
  notes: string;
  createdAt: string;
  updatedAt: string;
}

export interface ExtractedMetadata {
  researchQuestion: string;
  coreClaim: string;
  assumptions: string;
  theoreticalPerspective: string;
  methodology: string;
  findings: string;
  limitations: string;
  selfPositioning: string;
  version: number;
  lastUpdated: string;
  source: MetadataSource;
  isAiGenerated?: boolean;
}

export type MetadataSource =
  | 'manual'
  | 'ai'
  | 'crossref'
  | 'cnki'
  | 'arxiv'
  | 'openalex'
  | 'xmp'
  | 'filename'
  | 'mock';

export type RelationType =
  | 'supports'
  | 'opposes'
  | 'modifies'
  | 'adopts'
  | 'reinterprets';

export interface Relation {
  id: string;
  sourceId: string;
  targetId: string;
  type: RelationType;
  evidence: string;
  isManual: boolean;
  confidence?: number;
  createdAt: string;
  updatedAt: string;
}

export interface Annotation {
  id: string;
  paperId: string;
  field: keyof ExtractedMetadata | 'title' | 'abstract' | 'notes';
  text: string;
  pageNumber: number;
  rects: PdfRect[];
  createdAt: string;
}

export interface PdfRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface HypothesisNode {
  id: string;
  title: string;
  description: string;
  notes: string;
  relatedPaperIds: string[];
  createdAt: string;
  updatedAt: string;
}

export interface GraphLayout {
  locked: boolean;
  positions: Record<string, GraphPosition>;
}

export interface GraphPosition {
  x: number;
  y: number;
}

export interface ProjectSettings {
  activeAiProvider?: 'openai-compatible' | 'anthropic' | 'mock';
}
```

### 5.2 Rust 类型要求

在 `src-tauri/src/models/` 中定义等价结构体，使用：

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub year: Option<i32>,
    pub abstract_text: Option<String>,
    pub file_path: String,
    pub metadata: Option<ExtractedMetadata>,
    pub tags: Vec<String>,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}
```

注意：

- Rust 中不能使用 `abstract` 作为字段名，可使用 `abstract_text`，并通过 serde rename 映射为 `abstract`。
- 所有传给前端的数据必须使用 camelCase。
- 日期统一保存为 ISO 8601 字符串。

---

## 6. 项目文件格式

每个项目对应一个 `.guanfu` 文件夹。

```text
MyProject.guanfu/
├── project.json
└── papers/
    ├── paper-id-1.pdf
    ├── paper-id-2.pdf
    └── ...
```

### 6.1 要求

- 新建项目时由用户选择存放位置。
- 自动创建 `.guanfu` 文件夹。
- 自动创建 `project.json` 与 `papers/` 子目录。
- 导入 PDF 时复制源文件，不污染源文件。
- 复制后的 PDF 以 `{paperId}.pdf` 命名。
- 修改后自动保存到 `project.json`。
- 应用启动时尝试恢复上次打开项目。
- 上次打开项目路径可保存到 Tauri app config 目录中。

---

## 7. Tauri 后端架构

### 7.1 后端服务划分

后端 Rust 需要划分为以下服务：

#### ProjectService

负责：

- 新建项目
- 打开项目
- 保存项目
- 自动保存
- 读取最近打开项目
- 写入最近打开项目路径

#### PaperService

负责：

- 导入 PDF
- 复制文件到 `papers/`
- 删除文献
- 更新文献信息
- 获取 PDF 文件路径
- 读取 PDF 二进制或返回安全资源 URL

#### MetadataResolver

负责分级元数据抓取：

1. DOI → CrossRef
2. arXiv ID → arXiv API
3. CNKI 文件名 / 页眉页脚特征 → CNKI Resolver
4. PDF 前两页文本 → Crossref Query / OpenAlex 模糊匹配
5. XMP / 文件名兜底提取

#### AIManager

负责：

- OpenAI 兼容接口调用
- Anthropic Claude API 调用
- mock AI 返回
- 连接测试
- 文献解析
- 关系推荐
- 洞察生成

#### PdfTextExtractor

负责：

- PDF 全文文本抽取
- PDF 前几页文本抽取
- 为元数据抓取和 AI 解析提供文本

MVP 阶段如果 Rust PDF 解析复杂，可以先返回 mock 文本，但接口必须保留。

#### GraphService

负责：

- 关系增删改查
- 图谱布局保存
- 假设节点管理
- 修改日志
- 撤销 / 重做接口预留

#### InsightEngine

负责：

- 前端规则或后端规则均可，但推荐后端提供基础规则分析
- 发现潜在断裂带
- 发现缺乏多元检验的文献
- 发现方法单一的聚类
- 调用 AI 生成创造性洞察

---

## 8. Tauri Commands

所有前端与后端通信通过 `invoke()` 完成。

请实现以下命令，命名使用 snake_case。

### 8.1 项目命令

```rust
#[tauri::command]
async fn create_project(name: String, base_dir: String) -> Result<Project, AppError>;

#[tauri::command]
async fn open_project(project_path: String) -> Result<Project, AppError>;

#[tauri::command]
async fn save_project(project: Project) -> Result<(), AppError>;

#[tauri::command]
async fn get_recent_project() -> Result<Option<String>, AppError>;

#[tauri::command]
async fn set_recent_project(project_path: String) -> Result<(), AppError>;
```

### 8.2 文献命令

```rust
#[tauri::command]
async fn import_pdfs(project_path: String, file_paths: Vec<String>) -> Result<Vec<Paper>, AppError>;

#[tauri::command]
async fn update_paper(project_path: String, paper: Paper) -> Result<Paper, AppError>;

#[tauri::command]
async fn delete_paper(project_path: String, paper_id: String) -> Result<(), AppError>;

#[tauri::command]
async fn extract_pdf_text(project_path: String, paper_id: String) -> Result<String, AppError>;

#[tauri::command]
async fn get_pdf_file_url(project_path: String, paper_id: String) -> Result<String, AppError>;
```

### 8.3 元数据命令

```rust
#[tauri::command]
async fn resolve_metadata(project_path: String, paper_id: String) -> Result<MetadataResolveResult, AppError>;

#[tauri::command]
async fn search_metadata_candidates(project_path: String, paper_id: String) -> Result<Vec<MetadataCandidate>, AppError>;

#[tauri::command]
async fn apply_metadata_candidate(
    project_path: String,
    paper_id: String,
    candidate: MetadataCandidate
) -> Result<Paper, AppError>;
```

### 8.4 AI 命令

```rust
#[tauri::command]
async fn ai_parse_pdf(project_path: String, paper_id: String) -> Result<ExtractedMetadata, AppError>;

#[tauri::command]
async fn ai_recommend_relations(project_path: String) -> Result<Vec<RelationRecommendation>, AppError>;

#[tauri::command]
async fn ai_generate_insights(project_path: String) -> Result<Vec<Insight>, AppError>;

#[tauri::command]
async fn test_ai_connection(settings: AiSettings) -> Result<bool, AppError>;

#[tauri::command]
async fn save_ai_settings(settings: AiSettings) -> Result<(), AppError>;

#[tauri::command]
async fn get_ai_settings_masked() -> Result<MaskedAiSettings, AppError>;
```

### 8.5 图谱命令

```rust
#[tauri::command]
async fn add_relation(project_path: String, relation: Relation) -> Result<Relation, AppError>;

#[tauri::command]
async fn update_relation(project_path: String, relation: Relation) -> Result<Relation, AppError>;

#[tauri::command]
async fn delete_relation(project_path: String, relation_id: String) -> Result<(), AppError>;

#[tauri::command]
async fn save_graph_layout(project_path: String, layout: GraphLayout) -> Result<(), AppError>;

#[tauri::command]
async fn add_hypothesis(project_path: String, hypothesis: HypothesisNode) -> Result<HypothesisNode, AppError>;
```

### 8.6 洞察命令

```rust
#[tauri::command]
async fn run_insight_analysis(project_path: String) -> Result<Vec<Insight>, AppError>;
```

---

## 9. 前端 API 封装

不要在组件中直接大量调用 `invoke()`。

在 `src/api/` 中封装：

```ts
import { invoke } from '@tauri-apps/api/core';

export function createProject(name: string, baseDir: string) {
  return invoke<Project>('create_project', { name, baseDir });
}
```

要求：

- API 层负责命令名和参数映射。
- Store 调用 API 层。
- 组件调用 Store。
- 组件不关心 Tauri 细节。

---

## 10. 核心功能模块

---

# 10.1 项目与文献库管理

## 功能

- 新建项目
- 打开项目
- 恢复最近项目
- 导入 PDF
- 文献列表展示
- 文献筛选
- 文献排序
- 文献详情
- PDF 阅读视图
- 结构化数据视图

## 文献列表字段

显示：

- 首作者
- 年份
- 标题
- 标签
- 元数据状态
- AI 解析状态

## 交互

- 支持拖拽 PDF 到文献列表区域导入。
- 支持点击按钮选择 PDF。
- 选中文献后，主视图区可切换：
  - PDF 阅读
  - 结构化数据
  - 关系
  - 笔记

---

# 10.2 PDF 阅读与标注

## 要求

- 使用 `pdfjs-dist` 在前端渲染 PDF。
- 连续页滚动。
- 支持文本选择。
- 选中文字后弹出菜单。
- 可以将选中文本关联到某个元数据字段。
- 标注保存到 `project.json`。
- 点击标注引用后跳转回对应 PDF 页和位置。

## 字段关联

可关联字段包括：

- 标题
- 摘要
- 研究问题
- 核心主张
- 前提假设
- 理论视角
- 方法
- 主要发现
- 局限性
- 自述对话定位
- 我的笔记

---

# 10.3 元数据抓取

## 分级策略

`MetadataResolver` 必须按以下顺序尝试：

### 1. DOI / CrossRef

- 从文件名、PDF 前几页文本中提取 DOI。
- 若存在 DOI，调用 CrossRef REST API。
- 返回标题、作者、年份、摘要、期刊等。

### 2. arXiv

- 从文件名或文本中识别 arXiv ID。
- 调用 arXiv API。
- 返回标题、作者、年份、摘要。

### 3. CNKI

重点支持中文知网 PDF。

- 检测文件名中的知网特征。
- 检测页眉页脚中的知网特征。
- 参考 Jasminum 的思路：
  - 正则匹配 CNKI 文件名特征码。
  - 调用可用的公开题录查询方式。
  - 提取标题、作者、摘要、期刊、年份等。
- CNKI 真实接口不稳定时，先保留 Resolver 结构并提供 mock / fallback。

### 4. Crossref Query / OpenAlex 模糊匹配

- 若没有 DOI / arXiv / CNKI 结果：
  - 抽取前两页文本。
  - 通过规则或 AI 识别疑似标题。
  - 调用 Crossref Query API 或 OpenAlex API。
  - 返回多个候选项。
  - 用户确认后采纳。

### 5. XMP / 文件名兜底

- 全部失败时：
  - 尝试 PDF 内嵌元数据。
  - 尝试从文件名提取标题、作者、年份。
  - 标记为粗提取。
  - 提醒用户手动修正。

---

# 10.4 AI 智能解析

## 功能

对每篇论文提供 “AI 解析” 按钮，用于提取：

- 研究问题
- 核心主张
- 前提假设
- 理论视角
- 方法
- 主要发现
- 局限性
- 自述对话定位

## 要求

- AI 调用必须在 Rust 后端执行。
- 前端不得接触 API Key。
- 后端通过 `AIManager` 统一管理不同 provider。
- 支持：
  - OpenAI-compatible API
  - Anthropic Claude API
  - Mock Provider
- Prompt 模板存放在：

```text
src-tauri/src/prompts/
```

## 版本历史

- AI 结果自动填入字段。
- 字段旁显示 “AI 生成” 标签。
- 原始结果保存为历史版本。
- 预留回退与对比能力。

---

# 10.5 论点关系管理

## 关系类型

```ts
type RelationType =
  | 'supports'
  | 'opposes'
  | 'modifies'
  | 'adopts'
  | 'reinterprets';
```

中文映射：

```ts
const relationLabels = {
  supports: '支持',
  opposes: '反对',
  modifies: '修正 / 限定',
  adopts: '继承 / 采用',
  reinterprets: '再诠释',
};
```

## 建立方式

### 1. 图谱拖拽建联

- 从节点 A 拖拽连线至节点 B。
- 弹出关系创建对话框。
- 选择关系类型。
- 填写理由。
- 保存为 Relation。

### 2. 文献详情添加

- 在文献详情的“关系”标签页中添加。
- 通过下拉搜索选择另一篇文献。
- 指定关系类型和理由。

### 3. AI 推荐

- 点击 “发现关系”。
- 将文献结构化主张数据发送给后端。
- 后端调用 AI。
- 返回推荐关系列表：
  - sourceId
  - targetId
  - type
  - confidence
  - evidence
- 用户可单条确认或批量添加。

### 4. 编辑与纠错

- 点击图谱边或详情页关系条目。
- 弹出编辑窗。
- 可修改类型和理由。
- 预留撤销 / 重做。
- 保存修改日志。

---

# 10.6 图谱可视化

## 技术

使用 Cytoscape.js。

## 节点

- 极简圆形。
- 默认底色 `#EAEAEA`。
- 描边 `#333`。
- 选中描边 `#000` 并加粗。
- 文本为：首作者姓氏 + 年份。
- 文本过长自动截断。
- hover 展示完整标题、作者、年份、核心主张。
- 节点大小与关系数量正相关。
- 可根据理论流派使用低饱和背景色。
- 被多条反对 / 修正边指向时，边框变为黑色虚线，提示“争议文献”。

## 边

- 有方向箭头。
- 颜色按关系类型灰阶定义。
- 修正关系使用实线与短虚线交替样式。
- hover 显示 tooltip：
  - 关系类型
  - 理由摘要
- click 打开关系详情卡片。

## 布局

- 默认力导向布局。
- 提供 “锁定布局” 开关。
- 锁定后：
  - 用户拖拽节点位置被记忆。
  - 添加新节点 / 新关系时不触发全局重排。
  - 图谱位置保存到 `project.json` 的 `graphLayout.positions`。

## 争议热区

- 识别反对关系高度密集区域。
- 可以使用简单规则 MVP：
  - 某组节点之间 `opposes` 边数量超过阈值。
- 用浅灰半透明区域标出：

```css
rgba(0, 0, 0, 0.05)
```

- 使用极简虚线边框。
- 旁边生成可折叠 “争议摘要” 浮窗。
- 摘要可由规则生成，后续可接入 AI。

## 空白连接

在“洞察”模式下：

- 符合断裂空白条件的两个未连接节点之间显示极淡灰色闪烁虚线。
- 点击虚线显示说明。
- 可一键创建假设节点。

## 导出

支持导出：

- SVG
- PNG

---

# 10.7 假设节点管理

假设节点是用户基于空白发现创建的研究想法。

## 要求

- 图谱中使用虚线圆圈。
- 有专门面板列出所有假设节点。
- 可编辑：
  - 标题
  - 描述
  - 笔记
- 可与真实文献节点建立关系。
- 可正向或反向连接。

---

# 10.8 研究空白与机会点

点击 “洞察” 后执行分析，结果显示在右侧面板。

## 规则引擎 MVP

至少实现以下规则：

### 1. 潜在断裂带

同一理论视角下的文献之间没有任何关系。

### 2. 缺乏多元检验

某文献被高度支持，但几乎没有反对或修正。

### 3. 方法单一

某聚类或标签组中全部为质性研究，提示可补充定量证据。

## AI 洞察

可选。

- 将图谱结构整理成结构化文本。
- 后端调用 AI。
- 返回更有创造性的研究空白建议。
- 每条洞察可一键转为假设节点。

---

# 10.9 设置与 AI 模型接入

设置入口位于左侧导航齿轮图标。

## 支持 provider

### OpenAI Compatible

字段：

- enabled
- API Key
- Base URL
- Model

示例：

```text
https://api.openai.com/v1
gpt-4o
```

### Anthropic Claude

字段：

- enabled
- API Key
- Base URL，可选
- Model

示例：

```text
claude-3-5-sonnet-latest
```

### Mock

用于 MVP 和离线演示。

## 要求

- API Key 只保存在后端安全位置。
- 前端只能显示 masked key，例如：

```text
sk-****abcd
```

- 提供连接测试按钮。
- 连接测试由后端发送最小请求。
- Prompt 模板可从设置中导入自定义版本，MVP 可先预留入口。

---

## 11. 状态管理

使用 Pinia。

### projectStore

负责：

- 当前项目
- 项目路径
- 新建 / 打开 / 保存
- 自动保存
- 最近项目恢复

### paperStore

负责：

- 当前选中文献
- 文献列表
- 导入 PDF
- 更新文献
- 删除文献
- 元数据抓取
- AI 解析

### graphStore

负责：

- 关系列表
- 图谱节点
- 图谱边
- 当前选中节点 / 边
- 添加 / 编辑 / 删除关系
- 保存布局
- 假设节点

### insightStore

负责：

- 洞察列表
- 当前洞察
- 运行规则分析
- AI 洞察
- 洞察转假设节点

### settingsStore

负责：

- AI 设置
- 当前 provider
- 连接测试
- masked settings

---

## 12. 前端视图

使用 Vue Router 管理主视图切换。

推荐路由：

```ts
[
  { path: '/', redirect: '/library' },
  { path: '/library', component: LibraryView },
  { path: '/graph', component: GraphView },
  { path: '/insights', component: InsightsView },
  { path: '/settings', component: SettingsView },
]
```

### LibraryView

- 左侧文献列表
- 中间 PDF / 元数据 / 关系详情
- 右侧属性面板

### GraphView

- Cytoscape 图谱
- 图谱工具栏
- 右侧节点 / 边详情
- 争议摘要
- 假设节点面板

### InsightsView

- 洞察卡片
- 空白连接可视化
- 一键创建假设节点

### SettingsView

- AI provider 设置
- API Key 配置
- 连接测试
- Prompt 模板入口

---

## 13. 错误处理

后端统一定义 `AppError`。

要求：

- 所有 Tauri Command 返回 `Result<T, AppError>`。
- 错误需要可序列化给前端。
- 前端统一 toast 或 inline error 展示。
- 不允许 panic 暴露给用户。
- 网络失败、文件不存在、JSON 解析失败、AI 调用失败需要明确提示。

前端错误展示应保持克制，不使用花哨样式。

---

## 14. 自动保存

项目数据修改后自动保存。

建议策略：

- 前端 Store 中监听关键状态变化。
- 使用 debounce，例如 800ms。
- 调用 `save_project`。
- 保存中显示极简状态：
  - 已保存
  - 保存中
  - 保存失败

注意避免频繁写入大文件。

---

## 15. 开发顺序

请按以下顺序实现，不要一次性堆叠所有复杂功能。

### Phase 1：项目初始化与壳层

1. 初始化 Tauri + Vue + Vite + TypeScript 项目。
2. 配置 SCSS。
3. 建立基础三栏布局。
4. 建立全局设计变量。
5. 建立 Pinia 和 Router。
6. 建立 Tauri invoke API 封装。

### Phase 2：项目文件与文献导入

1. 实现 `create_project`。
2. 实现 `open_project`。
3. 实现 `save_project`。
4. 实现 `get_recent_project` / `set_recent_project`。
5. 实现 `import_pdfs`。
6. 前端完成文献列表和导入交互。
7. 实现自动保存。

### Phase 3：PDF 阅读

1. 集成 `pdfjs-dist`。
2. 实现 PDF 连续页渲染。
3. 实现文献选中后加载 PDF。
4. 实现文本选择弹出菜单。
5. 实现基础标注保存。

### Phase 4：元数据抓取

1. 实现 MetadataResolver 结构。
2. 实现 DOI 正则识别。
3. 实现 CrossRef 调用。
4. 实现 arXiv 识别与调用。
5. 实现 CNKI Resolver 框架。
6. 实现 fallback 文件名提取。
7. 前端实现抓取按钮、进度状态和结果采纳。

### Phase 5：结构化元数据编辑

1. 实现 MetadataEditor。
2. 支持 Markdown 文本编辑。
3. 支持手动保存。
4. 显示 source 和 AI 标签。
5. 支持我的笔记。

### Phase 6：AI 解析

1. 实现 AIManager。
2. 实现 mock provider。
3. 实现 OpenAI-compatible 请求。
4. 实现 Anthropic 请求。
5. 实现 `ai_parse_pdf`。
6. 前端接入 AI 解析按钮。
7. 保存历史版本接口预留。

### Phase 7：关系管理

1. 实现 Relation 数据结构。
2. 实现添加 / 编辑 / 删除关系。
3. 实现文献详情页关系管理。
4. 实现 AI 关系推荐 mock。
5. 实现确认推荐关系。

### Phase 8：图谱可视化

1. 集成 Cytoscape.js。
2. 将 papers / relations 转为 graph elements。
3. 实现关系类型样式。
4. 实现节点点击、边点击。
5. 实现拖拽建联。
6. 实现布局锁定与位置保存。
7. 实现 SVG / PNG 导出。

### Phase 9：洞察与假设节点

1. 实现规则引擎。
2. 实现洞察面板。
3. 实现洞察转假设节点。
4. 实现假设节点图谱显示。
5. 实现空白连接可视化。
6. 实现 AI 洞察接口预留。

### Phase 10：完善体验

1. 全局搜索。
2. 标签筛选。
3. 排序。
4. 批量选择。
5. 撤销 / 重做接口。
6. 错误处理完善。
7. 性能优化。
8. UI 细节打磨。

---

## 16. 代码风格

### Vue

- 统一使用 `<script setup lang="ts">`。
- 使用 Composition API。
- 组件 props 和 emits 必须有类型。
- 组件不直接处理复杂业务逻辑。
- 复杂逻辑放到 composables 或 stores。
- 样式使用 SCSS Modules 或 scoped SCSS。

### TypeScript

- 禁止滥用 `any`。
- 公共类型放在 `src/types/`。
- API 返回类型必须显式声明。
- relation、metadata source 等字段使用 union type。

### Rust

- 命令层只负责参数接收和返回。
- 业务逻辑放在 services。
- 数据结构放在 models。
- 错误统一使用 AppError。
- 网络请求封装在 client / resolver 中。
- 不要在 command 中堆复杂逻辑。

### 样式

- 不引入 Element Plus、Ant Design、Naive UI 等 UI 框架。
- 使用自定义基础组件。
- 保持黑白灰、低噪音、紧凑排版。
- 交互反馈清晰但克制。

---

## 17. MVP 允许 mock 的部分

以下功能 MVP 阶段可以先 mock，但必须有真实接口位置：

- AI 文献解析
- AI 关系推荐
- AI 洞察
- CNKI 精确解析
- PDF 后端全文抽取
- 争议摘要生成
- 社区检测算法

mock 数据必须结构真实，方便后续替换。

---

## 18. 必须优先保证的体验

1. 项目能创建、打开、保存。
2. PDF 能导入并显示在文献列表中。
3. PDF 能阅读。
4. 文献元数据能编辑和保存。
5. 文献之间能建立关系。
6. 图谱能展示节点和关系。
7. 图谱布局能保存。
8. 洞察面板能给出基础规则结果。
9. AI 配置不泄露到前端。
10. 整体 UI 符合现代黑白学术工具风格。

---

## 19. 不要做的事

- 不要使用 Electron。
- 不要把 API Key 放进前端。
- 不要使用大型 UI 框架。
- 不要把所有逻辑写进单个组件。
- 不要让组件直接操作文件系统。
- 不要破坏 `.guanfu` 项目结构。
- 不要在图谱中使用过多鲜艳颜色。
- 不要忽略错误处理。
- 不要生成无法编译的 TypeScript 或 Rust 代码。
- 不要一次性实现过多半成品功能，应按 Phase 顺序推进。

---

## 20. 当前开发目标

请从 Phase 1 开始，逐步实现：

1. Tauri + Vue 3 + Vite + TypeScript 项目基础结构。
2. 全局黑白设计系统。
3. 三栏式 AppShell。
4. Pinia / Router / API 封装。
5. Rust 后端 command 与 service 骨架。
6. Project / Paper / Relation / Metadata 基础类型。
7. 项目创建、打开、保存的最小闭环。

完成每个阶段后，确保项目可以运行、类型检查通过、基础功能可交互。

---

## 21. 开发环境注意事项

### 21.1 Cargo 命令必须指定 manifest-path

本项目的 `Cargo.toml` 位于 `src-tauri/` 子目录下，**不在项目根目录**。

所有 `cargo` 命令必须使用 `--manifest-path` 参数，否则会报 `could not find Cargo.toml` 错误：

```bash
# ✅ 正确
cargo check --manifest-path /Users/andrew/Desktop/Guanfu/src-tauri/Cargo.toml
cargo build --manifest-path /Users/andrew/Desktop/Guanfu/src-tauri/Cargo.toml

# ❌ 错误（项目根目录下没有 Cargo.toml）
cargo check
cargo build
```

### 21.2 npm / npx 命令在项目根目录运行

前端命令（`npx vue-tsc`、`npm run dev` 等）在项目根目录 `/Users/andrew/Desktop/Guanfu/` 下运行。

### 21.3 Rust 中禁止对 UTF-8 字符串做固定字节截断

中文等多字节 UTF-8 字符会 panic。**禁止**以下写法：

```rust
// ❌ 会 panic：30000 可能切到中文字符中间
&text[..30000]
&text[..text.len().min(300)]
```

**必须**使用 `truncate_str` 保证在字符边界截断：

```rust
// ✅ 正确
fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes { return s; }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) { end -= 1; }
    &s[..end]
}
```
```