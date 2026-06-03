use crate::errors::{AppError, AppResult};
use crate::models::{AiSettings, ExtractedMetadata, RelationRecommendation};
use tauri::Emitter;

/// Truncate a UTF-8 string to at most `max_bytes` bytes without splitting a character.
pub fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Generate language instruction based on user's UI locale setting.
fn language_instruction(locale: Option<&str>) -> &str {
    match locale {
        Some(l) if l == "sim" || l == "tra" || l.starts_with("zh") => "使用中文回答。",
        Some("eo") => "Respondu en Esperanto.",
        _ => "Respond in English.",
    }
}

const PARSE_PROMPT: &str = r#"你是一位社科文献分析专家。请仔细阅读以下论文全文或摘要，提取基本信息和 8 个维度的结构化分析。

## 严格要求

1. **基本信息字段**（title、authors、year、abstract）：从原文直接提取。如果确实找不到，title 填"未知标题"，authors 填["未知作者"]，year 填 null，abstract 填"未知"。不要编造基本信息。
2. **结构化分析字段**（researchQuestion 到 selfPositioning）：每个字段至少写 **3 句完整的句子**，要包含具体细节。即使原文没有明确表述，也要根据上下文和学科常识进行推断分析，不要写"无法推断"。
3. 不要引用原文原句，用自己的学术语言概括。
4. 如果 PDF 文本质量差或不完整，尽最大可能从已有文本中提取。

## JSON 字段说明（注意：返回 camelCase 字段名）

### 基本信息（从原文直接提取，找不到则用上述默认值）
- title: 论文标题，字符串。直接从原文提取完整标题，不要修改。找不到则填"未知标题"。
- authors: 作者列表，**必须是 JSON 字符串数组格式**，例如 `["张三", "李四", "Wang Wu"]`。提取所有作者姓名，保留原文格式。找不到则填["未知作者"]。
- year: 发表年份，整数。四位数年份，从期刊信息、页眉或引用格式中提取。找不到则填 null。
- abstract: 摘要，字符串。如原文有 Abstract 部分则完整提取；如没有，根据引言和结论自行撰写 4-6 句概括。实在无法撰写则填"未知"。

### 结构化分析（用自己的话深入分析，不允许留空）
- researchQuestion: 研究问题。这篇论文要解决什么具体问题？为什么这个问题在学科中有意义？问题的边界是什么？
- coreClaim: 核心主张。论文最核心的论点或发现是什么？它对现有知识的增量贡献在哪里？是否有反直觉的发现？
- assumptions: 前提假设。论文依赖哪些明示或隐含的假设？包括本体论预设（如理性选择假设）、方法论预设（如样本代表性）、情境假设（如特定文化背景）等。
- theoreticalPerspective: 理论视角。论文建立在哪些理论或框架之上？引用了哪些关键学者或经典文献？属于哪个学科分支或学派？是否有理论创新？
- methodology: 研究方法。具体使用了什么方法？必须包含：数据来源和收集方式、样本/案例规模和选取标准、分析工具或技术（如回归模型、扎根理论编码、实验设计）、效度控制措施。
- findings: 主要发现。报告了 3-5 条关键的实证或理论发现，每条应具体到变量关系、效应量或模式描述，不要只写"发现了显著差异"这种空话。
- limitations: 局限性。综合作者自述和你的判断，列出 3-4 条局限，包括方法论缺陷（如样本量不足）、外部效度（如只研究了单一国家）、理论覆盖不足、因果推断限制等。
- selfPositioning: 学术定位。论文如何与现有文献对话？明确指出它支持、反对、修正或扩展了哪些具体学者或流派的观点？在学术争论中它站在哪一边？它的独特定位是什么？

## 输出格式示例

```json
{
  "title": "具体论文标题",
  "authors": ["张三", "李四"],
  "year": 2023,
  "abstract": "本文研究了...通过...方法发现...",
  "researchQuestion": "本文试图回答的核心问题是...这一问题之所以重要，是因为...",
  "coreClaim": "本文的核心论点是...与已有研究不同的是，本文发现...",
  "assumptions": "本文建立在以下几个假设之上：第一，...；第二，...",
  "theoreticalPerspective": "本文以...理论为基础，借鉴了...的分析框架...",
  "methodology": "本文采用...方法，数据来自...样本规模为...使用...进行分析...",
  "findings": "第一，...。第二，...。第三，...",
  "limitations": "首先，...。其次，...。此外，...",
  "selfPositioning": "本文与...的观点形成对话，支持了...的研究结论，同时修正了...的理论..."
}
```

只返回 JSON 对象，不要任何解释、不要 markdown 代码块标记。

论文内容：
"#;

const FILL_EMPTY_PROMPT: &str = r#"以下是一篇学术论文的结构化分析结果，其中有些字段为空字符串。你的任务是根据已有字段的内容和学术常识，推断并补充空的结构化分析字段。

## 推断策略

- 从 researchQuestion 推断 coreClaim 和 theoreticalPerspective 的可能方向
- 从 methodology 推断 findings 和 limitations 的合理内容
- 从 theoreticalPerspective 推断 assumptions
- 从 coreClaim 和 findings 推断 selfPositioning
- 利用所有已有字段之间的逻辑关联进行交叉推断

## 严格要求

1. **基本信息字段**（title、authors、year、abstract）：保持原样，不要修改。如果它们是"未知标题"、["未知作者"]、null 或"未知"，就保持原样输出，不要编造。
2. **结构化分析字段**：每个空字段必须填写至少 3 句具体推断内容。
3. 不要写"根据已有信息无法推断"——必须给出合理的学术推断。
4. 推断内容必须符合学术规范，不要编造不存在的细节。
5. 输出完整的 JSON 对象，包含所有字段，已有的保持不变，空的用推断内容填充。

当前分析结果（JSON）：
"#;

pub async fn parse_text(text: &str, settings: &AiSettings) -> AppResult<ExtractedMetadata> {
    let provider = match settings.active_provider.as_deref() {
        Some("openaiCompatible") => "openaiCompatible",
        Some("anthropic") => "anthropic",
        _ => return Err(AppError::Unknown("未配置 AI 提供商".to_string())),
    };

    let locale = settings.locale.as_deref();
    let lang_instr = language_instruction(locale);

    // Pass 1: Main extraction
    let parse_prompt = format!("{}\n{}", lang_instr, PARSE_PROMPT);
    let mut metadata = match provider {
        "openaiCompatible" => {
            if let Some(ref config) = settings.openai_compatible {
                if !config.api_key.is_empty() {
                    call_openai(
                        config.api_key.as_str(),
                        config.base_url.as_deref().unwrap_or("https://api.openai.com/v1"),
                        config.model.as_str(),
                        text,
                        &parse_prompt,
                        4000,
                        locale,
                    ).await?
                } else {
                    return Err(AppError::Unknown("未配置 AI 提供商".to_string()));
                }
            } else {
                return Err(AppError::Unknown("未配置 AI 提供商".to_string()));
            }
        }
        "anthropic" => {
            if let Some(ref config) = settings.anthropic {
                if !config.api_key.is_empty() {
                    call_anthropic(
                        config.api_key.as_str(),
                        config.base_url.as_deref(),
                        config.model.as_str(),
                        text,
                        &parse_prompt,
                        4000,
                        locale,
                    ).await?
                } else {
                    return Err(AppError::Unknown("未配置 AI 提供商".to_string()));
                }
            } else {
                return Err(AppError::Unknown("未配置 AI 提供商".to_string()));
            }
        }
        _ => return Err(AppError::Unknown("未配置 AI 提供商".to_string())),
    };

    // Pass 2: Fill empty fields if any
    if has_empty_fields(&metadata) {
        let current_json = serde_json::to_string(&metadata).unwrap_or_default();
        let fill_prompt = format!("{}\n{}\n\n{}\n{}",
            lang_instr, FILL_EMPTY_PROMPT, current_json,
            if locale.map_or(false, |l| l == "sim" || l == "tra" || l.starts_with("zh")) { "请返回完整的 JSON 对象，所有字段都必须有内容：" }
            else { "Return the complete JSON object, all fields must have content:" }
        );

        if let Some(filled) = match provider {
            "openaiCompatible" => {
                if let Some(ref config) = settings.openai_compatible {
                    call_openai(
                        config.api_key.as_str(),
                        config.base_url.as_deref().unwrap_or("https://api.openai.com/v1"),
                        config.model.as_str(),
                        &fill_prompt,
                        "",
                        4000,
                        locale,
                    ).await.ok()
                } else { None }
            }
            "anthropic" => {
                if let Some(ref config) = settings.anthropic {
                    call_anthropic(
                        config.api_key.as_str(),
                        config.base_url.as_deref(),
                        config.model.as_str(),
                        &fill_prompt,
                        "",
                        4000,
                        locale,
                    ).await.ok()
                } else { None }
            }
            _ => None,
        } {
            metadata = merge_metadata(metadata, filled);
        }
    }

    metadata.version = 1;
    metadata.last_updated = chrono::Utc::now().to_rfc3339();
    metadata.source = "ai".to_string();
    metadata.is_ai_generated = Some(true);

    Ok(metadata)
}

fn has_empty_fields(m: &ExtractedMetadata) -> bool {
    m.title.as_ref().is_none_or(|s| s.trim().is_empty())
        || m.authors.as_ref().is_none_or(|a| a.is_empty())
        || m.year.is_none()
        || m.abstract_text.as_ref().is_none_or(|s| s.trim().is_empty())
        || m.research_question.trim().is_empty()
        || m.core_claim.trim().is_empty()
        || m.assumptions.trim().is_empty()
        || m.theoretical_perspective.trim().is_empty()
        || m.methodology.trim().is_empty()
        || m.findings.trim().is_empty()
        || m.limitations.trim().is_empty()
        || m.self_positioning.trim().is_empty()
}

fn merge_metadata(mut base: ExtractedMetadata, filled: ExtractedMetadata) -> ExtractedMetadata {
    if base.title.is_none() && filled.title.is_some() {
        base.title = filled.title;
    }
    if base.authors.is_none() && filled.authors.is_some() {
        base.authors = filled.authors;
    }
    if base.year.is_none() && filled.year.is_some() {
        base.year = filled.year;
    }
    if base.abstract_text.is_none() && filled.abstract_text.is_some() {
        base.abstract_text = filled.abstract_text;
    }
    if base.research_question.trim().is_empty() && !filled.research_question.trim().is_empty() {
        base.research_question = filled.research_question;
    }
    if base.core_claim.trim().is_empty() && !filled.core_claim.trim().is_empty() {
        base.core_claim = filled.core_claim;
    }
    if base.assumptions.trim().is_empty() && !filled.assumptions.trim().is_empty() {
        base.assumptions = filled.assumptions;
    }
    if base.theoretical_perspective.trim().is_empty() && !filled.theoretical_perspective.trim().is_empty() {
        base.theoretical_perspective = filled.theoretical_perspective;
    }
    if base.methodology.trim().is_empty() && !filled.methodology.trim().is_empty() {
        base.methodology = filled.methodology;
    }
    if base.findings.trim().is_empty() && !filled.findings.trim().is_empty() {
        base.findings = filled.findings;
    }
    if base.limitations.trim().is_empty() && !filled.limitations.trim().is_empty() {
        base.limitations = filled.limitations;
    }
    if base.self_positioning.trim().is_empty() && !filled.self_positioning.trim().is_empty() {
        base.self_positioning = filled.self_positioning;
    }
    base
}

async fn call_openai(api_key: &str, base_url: &str, model: &str, text: &str, prompt_prefix: &str, max_tokens: u32, locale: Option<&str>) -> AppResult<ExtractedMetadata> {
    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let truncated = truncate_str(text, 30000);

    let user_content = if prompt_prefix.is_empty() {
        truncated.to_string()
    } else {
        format!("{}{}", prompt_prefix, truncated)
    };

    let system_msg = format!("你是社科文献分析专家。严格只返回 JSON，不要任何其他文字。{}", language_instruction(locale));

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_msg},
            {"role": "user", "content": user_content}
        ],
        "temperature": 0.1,
        "max_tokens": max_tokens,
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Unknown(format!("AI 请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        return Err(AppError::Unknown(format!("AI API 错误 {}: {}", status, truncate_str(&err_text, 500))));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Unknown(format!("解析 AI 响应失败: {}", e)))?;

    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("{}");

    parse_ai_response(content)
}

async fn call_anthropic(api_key: &str, base_url: Option<&str>, model: &str, text: &str, prompt_prefix: &str, max_tokens: u32, locale: Option<&str>) -> AppResult<ExtractedMetadata> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v1/messages",
        base_url.unwrap_or("https://api.anthropic.com").trim_end_matches('/')
    );

    let truncated = truncate_str(text, 30000);

    let user_content = if prompt_prefix.is_empty() {
        truncated.to_string()
    } else {
        format!("{}{}", prompt_prefix, truncated)
    };

    let system_msg = format!("你是社科文献分析专家。严格只返回 JSON，不要任何其他文字。{}", language_instruction(locale));

    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": system_msg,
        "messages": [
            {"role": "user", "content": user_content}
        ],
    });

    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Unknown(format!("AI 请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        return Err(AppError::Unknown(format!("AI API 错误 {}: {}", status, truncate_str(&err_text, 500))));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Unknown(format!("解析 AI 响应失败: {}", e)))?;

    let content = resp_json["content"][0]["text"]
        .as_str()
        .unwrap_or("{}");

    parse_ai_response(content)
}

fn parse_ai_response(content: &str) -> AppResult<ExtractedMetadata> {
    let json_str = if let Some(start) = content.find('{') {
        if let Some(end) = content.rfind('}') {
            &content[start..=end]
        } else {
            content
        }
    } else {
        content
    };

    let v: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| {
            AppError::Unknown(format!("无法解析 AI 返回的 JSON: {}", e))
        })?;

    let get = |key: &str| -> String {
        v[key].as_str().unwrap_or("").to_string()
    };

    let get_authors = || -> Option<Vec<String>> {
        if let Some(arr) = v["authors"].as_array() {
            let authors: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .filter(|s| !s.trim().is_empty())
                .collect();
            if authors.is_empty() { None } else { Some(authors) }
        } else if let Some(s) = v["authors"].as_str() {
            // Handle comma/semicolon/slash separated string
            let authors: Vec<String> = s
                .split(|c: char| c == ',' || c == ';' || c == '/' || c == '、')
                .map(|a| a.trim().to_string())
                .filter(|a| !a.is_empty())
                .collect();
            if authors.is_empty() { None } else { Some(authors) }
        } else {
            None
        }
    };

    let get_year = || -> Option<i32> {
        v["year"].as_i64().map(|y| y as i32)
    };

    let metadata = ExtractedMetadata {
        title: if v["title"].is_string() { Some(get("title")) } else { None },
        authors: get_authors(),
        year: get_year(),
        abstract_text: if v["abstract"].is_string() { Some(get("abstract")) } else { None },
        research_question: get("researchQuestion"),
        core_claim: get("coreClaim"),
        assumptions: get("assumptions"),
        theoretical_perspective: get("theoreticalPerspective"),
        methodology: get("methodology"),
        findings: get("findings"),
        limitations: get("limitations"),
        self_positioning: get("selfPositioning"),
        version: 1,
        last_updated: chrono::Utc::now().to_rfc3339(),
        source: "ai".to_string(),
        is_ai_generated: Some(true),
    };
    Ok(metadata)
}

pub async fn test_connection(settings: &AiSettings) -> AppResult<bool> {
    match settings.active_provider.as_deref() {
        Some("openaiCompatible") => {
            if let Some(ref config) = settings.openai_compatible {
                if !config.api_key.is_empty() {
                    let client = reqwest::Client::new();
                    let url = format!(
                        "{}/chat/completions",
                        config.base_url.as_deref().unwrap_or("https://api.openai.com/v1").trim_end_matches('/')
                    );
                    let body = serde_json::json!({
                        "model": config.model,
                        "messages": [{"role": "user", "content": "Say OK"}],
                        "max_tokens": 5,
                    });
                    let resp = client
                        .post(&url)
                        .header("Authorization", format!("Bearer {}", config.api_key))
                        .header("Content-Type", "application/json")
                        .json(&body)
                        .send()
                        .await;
                    return Ok(resp.is_ok_and(|r| r.status().is_success()));
                }
            }
        }
        Some("anthropic") => {
            if let Some(ref config) = settings.anthropic {
                if !config.api_key.is_empty() {
                    let client = reqwest::Client::new();
                    let url = format!(
                        "{}/v1/messages",
                        config.base_url.as_deref().unwrap_or("https://api.anthropic.com").trim_end_matches('/')
                    );
                    let body = serde_json::json!({
                        "model": config.model,
                        "max_tokens": 5,
                        "messages": [{"role": "user", "content": "Say OK"}],
                    });
                    let resp = client
                        .post(&url)
                        .header("x-api-key", &config.api_key)
                        .header("anthropic-version", "2023-06-01")
                        .header("Content-Type", "application/json")
                        .json(&body)
                        .send()
                        .await;
                    return Ok(resp.is_ok_and(|r| r.status().is_success()));
                }
            }
        }
        _ => {}
    }

    Ok(false)
}

const RELATION_PROMPT: &str = r#"分析以下论文之间的学术论争关系。你的目标是发现显式和隐式的学术对话。

## 核心原则：一篇论文与另一篇论文可以存在多种关系

两篇论文之间往往不是单一关系。例如：
- A 在方法论上 supports B（继承了 B 的方法），但在结论上 opposes B（得出相反结论）
- A 在理论上 adopts B 的框架，同时又 modifies B 的某个具体假设
- A 对 B 的发现给出 reinterprets，同时 B 的另一个发现又被 A supports

**务必对每一对论文从多个角度审视，输出所有有意义的关系。**

## 六维发现策略（不仅看表面主张）

对每一对论文，从以下 6 个维度逐一审视：

1. **显式定位**：论文是否明确提到了对方？是支持、反对还是限定？
2. **隐含对话**：是否在讨论同一问题但未提及对方？研究问题是否有交集？
3. **方法论链**：一方的方法是否在另一方基础上发展？方法是否互补或对立？
4. **理论谱系**：是否属于同一理论传统的不同分支？引用的学者是否有交集？
5. **实证接力**：一方的发现是否为另一方提供了前提或验证？结论是否一致、矛盾或互补？
6. **时空对话**：在时间线上，后发表的是否回应了先发表的研究空白或局限？

## 分析步骤（对每对论文 A↔B，分别从 A→B 和 B→A 两个方向分析）
1. A 的核心主张是什么？B 的核心主张是什么？
2. 六维策略逐项检查
3. 分别从 A→B 和 B→A 两个方向判断，每个方向可能有不同关系
4. 对隐含关系也要输出，但 confidence 应如实标定

## 关系类型
- supports: A 的结论支持或印证 B
- opposes: A 的结论直接反驳或与 B 矛盾
- modifies: A 在 B 基础上修正、限定、扩展
- adopts: A 继承 B 的理论框架或方法
- reinterprets: A 对 B 的发现给出新诠释

## 发现依据类型（discoveryMethod）
- explicit-positioning: 论文明确提到了对方
- topic-overlap: 讨论同一问题但未提及对方
- methodology-chain: 方法论上的继承或发展
- theoretical-lineage: 理论框架的谱系关系
- empirical-relay: 实证发现的接力或验证
- temporal-response: 对先前研究的时间线回应

## 严格要求
1. 同一对论文可以有多条不同方向或不同类型的关系（如 A→B supports 同时 B→A opposes）
2. 不输出同一对论文的完全重复关系（sourceId+targetId+type 完全相同）
3. evidence 必须引用两篇论文的具体主张做对比，写出哪一点相同/不同，不要泛泛而谈
4. confidence 标准：显式关系 0.9+=明确定义，0.7+=较强证据，0.5+=合理推断；隐含关系 0.4+=合理推断，低于 0.4 不输出
5. 最多输出 25 条
6. discoveryMethod 标注每条关系的发现依据类型
7. **语言一致性**：evidence 字段必须完全使用回复语言书写。即使论文原文是其他语言，也要翻译为回复语言后再引用。禁止在同一句话中混用多种语言。

## 输出格式
只返回 JSON 数组，不要任何解释文字：
[{"sourceId":"A的id","targetId":"B的id","type":"关系类型","confidence":0.85,"evidence":"A主张...，B主张...，两者...","discoveryMethod":"发现依据类型"}]

## 论文列表
"#;

const RELATION_PROMPT_NEW: &str = r#"以下有「新论文」和「已有论文」两组。请分析每篇新论文与所有论文之间的学术论争关系。

## 核心原则：一篇论文与另一篇论文可以存在多种关系

两篇论文之间往往不是单一关系。例如：
- A 在方法论上 supports B，但在结论上 opposes B
- A 在理论上 adopts B 的框架，同时又 modifies B 的某个假设
- A→B 是 reinterprets，B→A 是 opposes

**务必从多个角度审视每一对论文，输出所有有意义的关系（包括双向）。**

## 六维发现策略（不仅看表面主张）

1. **显式定位**：论文是否明确提到了对方？
2. **隐含对话**：是否在讨论同一问题但未提及对方？研究问题是否有交集？
3. **方法论链**：一方的方法是否在另一方基础上发展？方法是否互补或对立？
4. **理论谱系**：是否属于同一理论传统的不同分支？引用的学者是否有交集？
5. **实证接力**：一方的发现是否为另一方提供了前提或验证？
6. **时空对话**：后发表的是否回应了先发表的研究空白或局限？

## 分析步骤（对每对论文，分别从 A→B 和 B→A 两个方向分析）
1. 方法论层面：是否互补、继承或对立？
2. 理论层面：框架是否相同、冲突或递进？
3. 结论层面：实证发现是否一致、矛盾或互补？
4. 六维策略逐项检查
5. 分别从新论文→已有论文、已有论文→新论文两个方向判断

## 关系类型
- supports: 结论支持或印证
- opposes: 结论直接反驳或矛盾
- modifies: 修正、限定、扩展
- adopts: 继承理论框架或方法
- reinterprets: 对发现给出新诠释

## 发现依据类型（discoveryMethod）
- explicit-positioning: 论文明确提到了对方
- topic-overlap: 讨论同一问题但未提及对方
- methodology-chain: 方法论上的继承或发展
- theoretical-lineage: 理论框架的谱系关系
- empirical-relay: 实证发现的接力或验证
- temporal-response: 对先前研究的时间线回应

## 严格要求
1. 同一对论文可以有多条不同方向或不同类型的关系
2. 不输出完全重复的关系
3. evidence 必须引用具体主张做对比
4. confidence 标准：显式关系 0.5+=合理推断；隐含关系 0.4+=合理推断；低于阈值不输出
5. 最多输出 25 条
6. discoveryMethod 标注每条关系的发现依据类型
7. **语言一致性**：evidence 字段必须完全使用回复语言书写。即使论文原文是其他语言，也要翻译为回复语言后再引用。禁止在同一句话中混用多种语言。

## 输出格式
只返回 JSON 数组：
[{"sourceId":"A的id","targetId":"B的id","type":"关系类型","confidence":0.85,"evidence":"具体对比分析","discoveryMethod":"发现依据类型"}]

## 新论文
"#;

/// Truncate a string to max_chars, adding "…" if truncated.
fn trunc(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}…", truncated)
    }
}

/// Build compact JSON summaries from papers, truncating long fields.
fn build_paper_summaries(papers: &[crate::models::Paper]) -> Vec<serde_json::Value> {
    papers.iter().filter_map(|paper| {
        let meta = paper.metadata.as_ref()?;
        Some(serde_json::json!({
            "id": paper.id,
            "title": trunc(&paper.title, 80),
            "year": paper.year,
            "claim": trunc(&meta.core_claim, 200),
            "theory": trunc(&meta.theoretical_perspective, 150),
            "method": trunc(&meta.methodology, 150),
            "findings": trunc(&meta.findings, 200),
            "position": trunc(&meta.self_positioning, 150),
        }))
    }).collect()
}

async fn call_ai_raw(prompt: &str, settings: &AiSettings) -> AppResult<String> {
    let locale = settings.locale.as_deref();
    call_ai_raw_with_tokens(prompt, settings, 12000, locale).await
}

async fn call_ai_raw_with_tokens(prompt: &str, settings: &AiSettings, max_tokens: u32, locale: Option<&str>) -> AppResult<String> {
    match settings.active_provider.as_deref() {
        Some("openaiCompatible") => {
            if let Some(ref config) = settings.openai_compatible {
                if !config.api_key.is_empty() {
                    call_openai_raw(
                        config.api_key.as_str(),
                        config.base_url.as_deref().unwrap_or("https://api.openai.com/v1"),
                        config.model.as_str(),
                        prompt,
                        max_tokens,
                        locale,
                    ).await
                } else {
                    Err(AppError::Unknown("未配置 AI 提供商".to_string()))
                }
            } else {
                Err(AppError::Unknown("未配置 AI 提供商".to_string()))
            }
        }
        Some("anthropic") => {
            if let Some(ref config) = settings.anthropic {
                if !config.api_key.is_empty() {
                    call_anthropic_raw(
                        config.api_key.as_str(),
                        config.base_url.as_deref(),
                        config.model.as_str(),
                        prompt,
                        max_tokens,
                        locale,
                    ).await
                } else {
                    Err(AppError::Unknown("未配置 AI 提供商".to_string()))
                }
            } else {
                Err(AppError::Unknown("未配置 AI 提供商".to_string()))
            }
        }
        _ => Err(AppError::Unknown("未配置 AI 提供商".to_string())),
    }
}

pub async fn recommend_relations(
    papers_with_meta: &[crate::models::Paper],
    settings: &AiSettings,
) -> AppResult<Vec<RelationRecommendation>> {
    let summaries = build_paper_summaries(papers_with_meta);
    if summaries.len() < 2 {
        return Ok(vec![]);
    }

    let papers_json = serde_json::to_string(&summaries)
        .map_err(|e| AppError::Unknown(format!("序列化论文数据失败: {}", e)))?;

    let locale = settings.locale.as_deref();
    let lang_instr = language_instruction(locale);
    let full_prompt = format!("{}\n{}\n{}", lang_instr, RELATION_PROMPT, papers_json);
    eprintln!("[relations] 全量推荐: {} 篇论文, prompt {} 字符", summaries.len(), full_prompt.len());
    let content = call_ai_raw(&full_prompt, settings).await?;
    eprintln!("[relations] AI 返回 {} 字符", content.len());
    let result = parse_relation_response(&content, papers_with_meta)?;
    eprintln!("[relations] 解析出 {} 条关系", result.len());
    Ok(result)
}

pub async fn recommend_relations_for_new(
    new_papers: &[crate::models::Paper],
    all_papers: &[crate::models::Paper],
    settings: &AiSettings,
) -> AppResult<Vec<RelationRecommendation>> {
    let new_summaries = build_paper_summaries(new_papers);
    let existing_summaries = build_paper_summaries(
        &all_papers.iter()
            .filter(|p| !new_papers.iter().any(|np| np.id == p.id))
            .cloned()
            .collect::<Vec<_>>()
    );

    if new_summaries.is_empty() {
        return Ok(vec![]);
    }

    let locale = settings.locale.as_deref();
    let lang_instr = language_instruction(locale).to_string();

    // If new papers fit in a single call (≤12), use single call
    const CHUNK_SIZE: usize = 12;
    if new_summaries.len() <= CHUNK_SIZE {
        let mut prompt = format!("{}\n{}", lang_instr, RELATION_PROMPT_NEW);
        prompt.push_str(&serde_json::to_string(&new_summaries).unwrap_or_default());
        if !existing_summaries.is_empty() {
            prompt.push_str("\n\n## 已有论文\n");
            prompt.push_str(&serde_json::to_string(&existing_summaries).unwrap_or_default());
        }
        eprintln!("[relations] 新论文推荐: {} 新 / {} 已有, prompt {} 字符",
            new_summaries.len(), existing_summaries.len(), prompt.len());
        let content = call_ai_raw(&prompt, settings).await?;
        eprintln!("[relations] AI 返回 {} 字符", content.len());
        let result = parse_relation_response(&content, all_papers)?;
        eprintln!("[relations] 解析出 {} 条关系", result.len());
        return Ok(result);
    }

    // Chunk new papers and run multiple calls in parallel
    let chunk_count = (new_summaries.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;
    eprintln!("[relations] 新论文推荐(分块): {} 新 / {} 已有, 分 {} 块",
        new_summaries.len(), existing_summaries.len(), chunk_count);
    let mut all_results = Vec::new();
    let mut handles = Vec::new();
    let owned_settings = settings.clone();

    for chunk in new_summaries.chunks(CHUNK_SIZE) {
        let chunk = chunk.to_vec();
        let existing = existing_summaries.clone();
        let lang = lang_instr.clone();
        let s = owned_settings.clone();
        handles.push(tokio::spawn(async move {
            let mut prompt = format!("{}\n{}", lang, RELATION_PROMPT_NEW);
            prompt.push_str(&serde_json::to_string(&chunk).unwrap_or_default());
            if !existing.is_empty() {
                prompt.push_str("\n\n## 已有论文\n");
                prompt.push_str(&serde_json::to_string(&existing).unwrap_or_default());
            }
            call_ai_raw(&prompt, &s).await
        }));
    }

    for h in handles {
        if let Ok(Ok(content)) = h.await {
            eprintln!("[relations] 分块返回 {} 字符", content.len());
            match parse_relation_response(&content, all_papers) {
                Ok(relations) => {
                    eprintln!("[relations] 分块解析出 {} 条关系", relations.len());
                    all_results.extend(relations);
                }
                Err(e) => {
                    eprintln!("[relations] 分块解析失败: {}", e);
                }
            }
        }
    }

    // Deduplicate by (sourceId, targetId, type)
    all_results.sort_by(|a, b| {
        (&a.source_id, &a.target_id, &a.r#type)
            .cmp(&(&b.source_id, &b.target_id, &b.r#type))
    });
    all_results.dedup_by(|a, b| {
        a.source_id == b.source_id && a.target_id == b.target_id && a.r#type == b.r#type
    });

    Ok(all_results)
}

/// Call OpenAI and return raw text content (not parsed as ExtractedMetadata).
async fn call_openai_raw(api_key: &str, base_url: &str, model: &str, prompt: &str, max_tokens: u32, locale: Option<&str>) -> AppResult<String> {
    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let system_msg = format!("你是社科学术论争分析专家。严格只返回 JSON，不要任何其他文字。所有文本字段（尤其是 evidence）必须使用同一种语言，禁止混用多种语言。{}", language_instruction(locale));

    eprintln!("[relations] OpenAI 请求: model={}, prompt={} 字符, max_tokens={}", model, prompt.len(), max_tokens);

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_msg},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.2,
        "max_tokens": max_tokens,
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Unknown(format!("AI 请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        eprintln!("[relations] OpenAI 错误: status={}, body={}", status, truncate_str(&err_text, 300));
        return Err(AppError::Unknown(format!("AI API 错误 {}: {}", status, err_text)));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Unknown(format!("解析 AI 响应失败: {}", e)))?;

    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("[]");
    let finish_reason = resp_json["choices"][0]["finish_reason"].as_str().unwrap_or("unknown");
    eprintln!("[relations] OpenAI 响应: {} 字符, finish_reason={}", content.len(), finish_reason);

    Ok(content.to_string())
}

/// Call Anthropic and return raw text content.
async fn call_anthropic_raw(api_key: &str, base_url: Option<&str>, model: &str, prompt: &str, max_tokens: u32, locale: Option<&str>) -> AppResult<String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v1/messages",
        base_url.unwrap_or("https://api.anthropic.com").trim_end_matches('/')
    );

    let system_msg = format!("你是社科学术论争分析专家。严格只返回 JSON，不要任何其他文字。所有文本字段（尤其是 evidence）必须使用同一种语言，禁止混用多种语言。{}", language_instruction(locale));

    eprintln!("[relations] Anthropic 请求: model={}, prompt={} 字符, max_tokens={}", model, prompt.len(), max_tokens);

    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": system_msg,
        "messages": [
            {"role": "user", "content": prompt}
        ],
    });

    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Unknown(format!("AI 请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        eprintln!("[relations] Anthropic 错误: status={}, body={}", status, truncate_str(&err_text, 300));
        return Err(AppError::Unknown(format!("AI API 错误 {}: {}", status, err_text)));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Unknown(format!("解析 AI 响应失败: {}", e)))?;

    let content = resp_json["content"][0]["text"]
        .as_str()
        .unwrap_or("[]");
    let stop_reason = resp_json["stop_reason"].as_str().unwrap_or("unknown");
    eprintln!("[relations] Anthropic 响应: {} 字符, stop_reason={}", content.len(), stop_reason);

    Ok(content.to_string())
}

fn parse_relation_response(content: &str, papers: &[crate::models::Paper]) -> AppResult<Vec<RelationRecommendation>> {
    eprintln!("[relations] 解析响应: {} 字符, 前200字: {:?}", content.len(), truncate_str(content, 200));
    let json_str = if let Some(start) = content.find('[') {
        if let Some(end) = content.rfind(']') {
            &content[start..=end]
        } else {
            eprintln!("[relations] 警告: 找到 '[' 但未找到 ']'");
            content
        }
    } else if let Some(start) = content.find('{') {
        // Some models return a single object or wrap in {"relations": [...]}
        if let Some(end) = content.rfind('}') {
            let wrapper: serde_json::Value = serde_json::from_str(&content[start..=end])
                .map_err(|e| AppError::Unknown(format!("无法解析 AI 返回的 JSON: {}", e)))?;
            if let Some(arr) = wrapper.get("relations").and_then(|v| v.as_array()) {
                eprintln!("[relations] 从 {{relations:[...]}} 包装中提取");
                return parse_relation_array(arr, papers);
            }
            &content[start..=end]
        } else {
            content
        }
    } else {
        eprintln!("[relations] 警告: 响应中未找到 '[' 或 '{{'");
        content
    };

    // Try parsing as-is first
    match serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
        Ok(arr) => parse_relation_array(&arr, papers),
        Err(e) => {
            eprintln!("[relations] JSON 解析失败: {}, 尝试截断恢复...", e);
            // JSON may be truncated (e.g. EOF mid-string) — try to recover
            if let Some(last_brace) = json_str.rfind('}') {
                let truncated = &json_str[..=last_brace];
                let recovered = format!("{}]", truncated);
                match serde_json::from_str::<Vec<serde_json::Value>>(&recovered) {
                    Ok(arr) => {
                        eprintln!("[relations] 截断恢复成功: {} 条关系", arr.len());
                        parse_relation_array(&arr, papers)
                    }
                    Err(e2) => {
                        eprintln!("[relations] 截断恢复也失败: {}", e2);
                        Err(AppError::Unknown(format!("无法解析 AI 返回的关系 JSON: {}", e2)))
                    }
                }
            } else {
                Err(AppError::Unknown(format!("无法解析 AI 返回的关系 JSON: {}", e)))
            }
        }
    }
}

fn parse_relation_array(arr: &[serde_json::Value], papers: &[crate::models::Paper]) -> AppResult<Vec<RelationRecommendation>> {
    let paper_ids: std::collections::HashSet<&str> = papers.iter().map(|p| p.id.as_str()).collect();
    let valid_types = ["supports", "opposes", "modifies", "adopts", "reinterprets"];

    let mut skipped_unknown_paper = 0;
    let mut skipped_self = 0;
    let mut skipped_bad_type = 0;
    let mut skipped_low_conf = 0;

    let recommendations: Vec<RelationRecommendation> = arr
        .iter()
        .filter_map(|item| {
            let source_id = item["sourceId"].as_str()?;
            let target_id = item["targetId"].as_str()?;
            let rel_type = item["type"].as_str()?;
            let evidence = item["evidence"].as_str().unwrap_or("");
            let confidence = item["confidence"].as_f64().unwrap_or(0.5);

            // Validate
            if !paper_ids.contains(source_id) || !paper_ids.contains(target_id) {
                skipped_unknown_paper += 1;
                eprintln!("[relations] 跳过: 未知论文 source={} target={}", source_id, target_id);
                return None;
            }
            if source_id == target_id {
                skipped_self += 1;
                return None;
            }
            if !valid_types.contains(&rel_type) {
                skipped_bad_type += 1;
                eprintln!("[relations] 跳过: 无效类型 '{}' (source={} target={})", rel_type, source_id, target_id);
                return None;
            }
            if confidence < 0.5 {
                skipped_low_conf += 1;
                return None;
            }

            Some(RelationRecommendation {
                source_id: source_id.to_string(),
                target_id: target_id.to_string(),
                r#type: rel_type.to_string(),
                confidence,
                evidence: evidence.to_string(),
                discovery_method: item["discoveryMethod"].as_str().map(String::from),
            })
        })
        .collect();

    let total_skipped = arr.len() - recommendations.len();
    if total_skipped > 0 {
        eprintln!("[relations] 过滤: 原始 {} 条, 有效 {} 条, 跳过 {} 条 (未知论文={}, 自引用={}, 无效类型={}, 低置信度={})",
            arr.len(), recommendations.len(), total_skipped,
            skipped_unknown_paper, skipped_self, skipped_bad_type, skipped_low_conf);
    }

    Ok(recommendations)
}

const INSIGHT_PROMPT: &str = r#"你是一位社科研究方法论专家。请分析以下论文图谱，发现研究空白与机会。

## 分析维度

1. **断裂带**：哪些论文之间应该有论争关系但实际上没有？它们的研究问题或理论视角是否有交集但未被对话？
2. **缺乏多元检验**：哪些论点被过度接受（只有支持，没有反对/修正）？是否存在需要被质疑的"共识"？
3. **方法单一**：某一组论文是否都用同一种方法？缺少哪些方法论视角的补充？
4. **理论空白**：当前图谱中是否存在明显缺失的理论流派或研究视角？
5. **创造性机会**：基于以上分析，有哪些新的研究问题值得探索？

## 严格要求

1. 每条洞察必须具体，引用论文标题和具体内容，不要泛泛而谈
2. 最多输出 5 条最有价值的洞察
3. 不要重复规则引擎已经能发现的简单断裂带

## 输出格式

只返回 JSON 数组：
[{"type":"insight类型","title":"简短标题","description":"详细分析","relatedPaperIds":["id1","id2"]}]

类型可以是：potential-fault-line / lack-pluralistic-testing / method-homogeneity / theoretical-gap / creative-opportunity

## 论文数据
"#;

pub async fn generate_insights(
    project: &crate::models::Project,
    settings: &AiSettings,
) -> AppResult<Vec<crate::models::Insight>> {
    let papers_with_meta: Vec<_> = project
        .papers
        .iter()
        .filter(|p| p.metadata.is_some())
        .collect();

    if papers_with_meta.len() < 2 {
        return Ok(vec![]);
    }

    // Build structured data for AI
    let papers_data: Vec<serde_json::Value> = papers_with_meta.iter().map(|paper| {
        let meta = paper.metadata.as_ref().unwrap();
        serde_json::json!({
            "id": paper.id,
            "title": trunc(&paper.title, 100),
            "year": paper.year,
            "researchQuestion": trunc(&meta.research_question, 200),
            "coreClaim": trunc(&meta.core_claim, 200),
            "theoreticalPerspective": trunc(&meta.theoretical_perspective, 200),
            "methodology": trunc(&meta.methodology, 200),
            "findings": trunc(&meta.findings, 200),
            "limitations": trunc(&meta.limitations, 150),
            "selfPositioning": trunc(&meta.self_positioning, 150),
        })
    }).collect();

    let relations_data: Vec<serde_json::Value> = project.relations.iter().map(|r| {
        serde_json::json!({
            "sourceId": r.source_id,
            "targetId": r.target_id,
            "type": r.r#type,
        })
    }).collect();

    let graph_json = serde_json::json!({
        "papers": papers_data,
        "relations": relations_data,
    });

    let locale = settings.locale.as_deref();
    let lang_instr = language_instruction(locale);
    let full_prompt = format!("{}\n{}\n{}", lang_instr, INSIGHT_PROMPT, serde_json::to_string(&graph_json).unwrap_or_default());
    let content = call_ai_raw(&full_prompt, settings).await?;

    // Parse response
    let json_str = if let Some(start) = content.find('[') {
        if let Some(end) = content.rfind(']') {
            &content[start..=end]
        } else {
            return Ok(vec![]);
        }
    } else {
        return Ok(vec![]);
    };

    let arr: Vec<serde_json::Value> = serde_json::from_str(json_str)
        .map_err(|e| AppError::Unknown(format!("无法解析 AI 洞察 JSON: {}", e)))?;

    let paper_ids: std::collections::HashSet<&str> = project.papers.iter().map(|p| p.id.as_str()).collect();

    let insights: Vec<crate::models::Insight> = arr.iter().filter_map(|item| {
        let insight_type = item["type"].as_str().unwrap_or("ai-insight");
        let title = item["title"].as_str().unwrap_or("AI 洞察");
        let description = item["description"].as_str().unwrap_or("");

        let related: Vec<String> = item["relatedPaperIds"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let related: Vec<String> = related.into_iter()
            .filter(|id| paper_ids.contains(id.as_str()))
            .collect();

        if description.is_empty() {
            return None;
        }

        Some(crate::models::Insight {
            id: uuid::Uuid::new_v4().to_string(),
            r#type: insight_type.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            related_paper_ids: related,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }).collect();

    Ok(insights)
}

/// Call AI for conversational chat with a custom system prompt. Returns raw text.
pub async fn call_chat(user_prompt: &str, settings: &AiSettings) -> AppResult<String> {
    let locale = settings.locale.as_deref();
    let lang_instr = language_instruction(locale);
    let chat_system = format!("{}\n{}", CHAT_SYSTEM_PROMPT, lang_instr);

    match settings.active_provider.as_deref() {
        Some("openaiCompatible") => {
            if let Some(ref config) = settings.openai_compatible {
                if !config.api_key.is_empty() {
                    call_openai_with_system(
                        config.api_key.as_str(),
                        config.base_url.as_deref().unwrap_or("https://api.openai.com/v1"),
                        config.model.as_str(),
                        &chat_system,
                        user_prompt,
                        4000,
                    ).await
                } else {
                    Err(AppError::Unknown("未配置 AI 提供商".to_string()))
                }
            } else {
                Err(AppError::Unknown("未配置 AI 提供商".to_string()))
            }
        }
        Some("anthropic") => {
            if let Some(ref config) = settings.anthropic {
                if !config.api_key.is_empty() {
                    call_anthropic_with_system(
                        config.api_key.as_str(),
                        config.base_url.as_deref(),
                        config.model.as_str(),
                        &chat_system,
                        user_prompt,
                        4000,
                    ).await
                } else {
                    Err(AppError::Unknown("未配置 AI 提供商".to_string()))
                }
            } else {
                Err(AppError::Unknown("未配置 AI 提供商".to_string()))
            }
        }
        _ => Err(AppError::Unknown("未配置 AI 提供商".to_string())),
    }
}

const CHAT_SYSTEM_PROMPT: &str = r#"你是"观复"——一款学术文献论争图谱工具中的知识库助手。用户正在研究一批学术论文，你可以基于论文内容、论争关系和研究洞察来回答问题。

## 回答要求
1. 基于提供的论文内容回答，不要编造不存在的信息
2. 引用具体论文标题和内容来支撑论点
3. 如果涉及论文间的论争关系，明确指出支持/反对/修正等关系
4. 如果涉及研究空白或机会，可以引用洞察分析结果
5. 如果提供的上下文无法回答问题，诚实说明
6. 使用 Markdown 格式排版回答，可使用标题、列表、加粗、引用等格式使内容更清晰易读"#;

async fn call_openai_with_system(
    api_key: &str,
    base_url: &str,
    model: &str,
    system_prompt: &str,
    user_content: &str,
    max_tokens: u32,
) -> AppResult<String> {
    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_content}
        ],
        "temperature": 0.3,
        "max_tokens": max_tokens,
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Unknown(format!("AI 请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        return Err(AppError::Unknown(format!("AI API 错误 {}: {}", status, err_text)));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Unknown(format!("解析 AI 响应失败: {}", e)))?;

    Ok(resp_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("无法生成回答")
        .to_string())
}

async fn call_anthropic_with_system(
    api_key: &str,
    base_url: Option<&str>,
    model: &str,
    system_prompt: &str,
    user_content: &str,
    max_tokens: u32,
) -> AppResult<String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v1/messages",
        base_url.unwrap_or("https://api.anthropic.com").trim_end_matches('/')
    );

    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": system_prompt,
        "messages": [
            {"role": "user", "content": user_content}
        ],
    });

    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Unknown(format!("AI 请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        return Err(AppError::Unknown(format!("AI API 错误 {}: {}", status, err_text)));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Unknown(format!("解析 AI 响应失败: {}", e)))?;

    Ok(resp_json["content"][0]["text"]
        .as_str()
        .unwrap_or("无法生成回答")
        .to_string())
}

/// Streaming version of call_chat — emits `chat-stream` events via Tauri.
pub async fn call_chat_stream(
    user_prompt: &str,
    settings: &AiSettings,
    app_handle: &tauri::AppHandle,
) -> AppResult<String> {
    let locale = settings.locale.as_deref();
    let lang_instr = language_instruction(locale);
    let chat_system = format!("{}\n{}", CHAT_SYSTEM_PROMPT, lang_instr);

    match settings.active_provider.as_deref() {
        Some("openaiCompatible") => {
            if let Some(ref config) = settings.openai_compatible {
                if !config.api_key.is_empty() {
                    call_openai_stream(
                        app_handle,
                        config.api_key.as_str(),
                        config.base_url.as_deref().unwrap_or("https://api.openai.com/v1"),
                        config.model.as_str(),
                        &chat_system,
                        user_prompt,
                        4000,
                    ).await
                } else {
                    Err(AppError::Unknown("未配置 AI 提供商".to_string()))
                }
            } else {
                Err(AppError::Unknown("未配置 AI 提供商".to_string()))
            }
        }
        Some("anthropic") => {
            if let Some(ref config) = settings.anthropic {
                if !config.api_key.is_empty() {
                    call_anthropic_stream(
                        app_handle,
                        config.api_key.as_str(),
                        config.base_url.as_deref(),
                        config.model.as_str(),
                        &chat_system,
                        user_prompt,
                        4000,
                    ).await
                } else {
                    Err(AppError::Unknown("未配置 AI 提供商".to_string()))
                }
            } else {
                Err(AppError::Unknown("未配置 AI 提供商".to_string()))
            }
        }
        _ => Err(AppError::Unknown("未配置 AI 提供商".to_string())),
    }
}

async fn call_openai_stream(
    app_handle: &tauri::AppHandle,
    api_key: &str,
    base_url: &str,
    model: &str,
    system_prompt: &str,
    user_content: &str,
    max_tokens: u32,
) -> AppResult<String> {
    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_content}
        ],
        "temperature": 0.3,
        "max_tokens": max_tokens,
        "stream": true,
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Unknown(format!("AI 请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        return Err(AppError::Unknown(format!("AI API 错误 {}: {}", status, err_text)));
    }

    let mut full_content = String::new();
    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| AppError::Unknown(format!("读取流失败: {}", e)))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Process complete lines
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim().to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if line.is_empty() { continue; }

            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" { break; }
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                        full_content.push_str(content);
                        let _ = app_handle.emit("chat-stream", serde_json::json!({"content": content}));
                    }
                }
            }
        }
    }

    if full_content.is_empty() {
        Ok("无法生成回答".to_string())
    } else {
        Ok(full_content)
    }
}

async fn call_anthropic_stream(
    app_handle: &tauri::AppHandle,
    api_key: &str,
    base_url: Option<&str>,
    model: &str,
    system_prompt: &str,
    user_content: &str,
    max_tokens: u32,
) -> AppResult<String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v1/messages",
        base_url.unwrap_or("https://api.anthropic.com").trim_end_matches('/')
    );

    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": system_prompt,
        "messages": [
            {"role": "user", "content": user_content}
        ],
        "stream": true,
    });

    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Unknown(format!("AI 请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        return Err(AppError::Unknown(format!("AI API 错误 {}: {}", status, err_text)));
    }

    let mut full_content = String::new();
    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| AppError::Unknown(format!("读取流失败: {}", e)))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Process complete lines
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim().to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if line.is_empty() { continue; }

            if let Some(event_type) = line.strip_prefix("event: ") {
                // Look ahead for data line (Anthropic sends event then data on separate lines)
                if event_type == "content_block_delta" {
                    // Data will be on the next line(s), handled below
                }
            } else if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(text_content) = parsed["delta"]["text"].as_str() {
                        full_content.push_str(text_content);
                        let _ = app_handle.emit("chat-stream", serde_json::json!({"content": text_content}));
                    }
                }
            }
        }
    }

    if full_content.is_empty() {
        Ok("无法生成回答".to_string())
    } else {
        Ok(full_content)
    }
}
