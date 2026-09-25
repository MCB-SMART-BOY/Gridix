//! check-doc-symbols — 校验文档反引号中引用的项目符号确实存在于源码。
//!
//! 背景：`.claude/` 文档曾出现 `DialogStyle::centered_shortcut`、`FooterResult::Dismissed`
//! 这类代码中并不存在的 API，只靠人工复核才发现。本校验器把"文档引用的符号必须存在"
//! 变成可执行门禁。
//!
//! 判定规则（保守，宁可漏报不可误报）：
//! - 只检查反引号片段，且跳过围栏代码块（示例代码可以是伪代码）。
//! - 片段必须逐段匹配标识符，可带结尾 `()`：`Type::member`、`Type::member()`、`bare_name`；
//!   含空白或参数列表的片段（`Message::GridSaveDone { … }`、`f(a, b)`）不参与判定。
//! - 首段小写的多段路径视为外部 crate 或子模块路径（`egui::Order::Foreground`），跳过。
//! - 裸标识符只在形态明确时检查：蛇形（`show_blocking`）或大驼峰（`DialogStyle`）。
//! - 文件引用必须路径限定（含 `/`）且以 [`REPO_FILE_EXTENSIONS`] 中的扩展名结尾，
//!   按后缀匹配，因此 `app/runtime/handler.rs` 与 `src/app/runtime/handler.rs` 都成立。
//!   裸文件名（`keymap.toml`、`settings.json`）、`self.sql` 这类字段访问、命令/版本号、
//!   以及含空格或 glob 的片段都不判为文件引用。
//!   仓库外的合法路径前缀见 [`EXTERNAL_PATH_PREFIXES`]（发布流程描述的 nixpkgs 位置）。
//! - 行内含 `doc-symbols: ignore` 的行整行跳过；整文件豁免必须是含
//!   `doc-symbols: ignore-file` 的 HTML 注释行，避免散文提及静默关闭整份文件。
//! - 符号索引覆盖 `src/`、`tests/` 的代码与文件名主干，按行剥离 `//` 注释，
//!   并排除本文件自身（其测试按设计包含并不存在的样例符号）。
//!
//! 用法：`cargo run --bin check-doc-symbols`
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::LazyLock;

/// 反引号片段。
static SPAN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`([^`]+)`").expect("span regex"));

/// 源码中的标识符（无需解析语法：任何出现过的名字都算已知）。
static IDENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z_][A-Za-z0-9_]*").expect("ident regex"));

/// 免检行标记。
const IGNORE_MARKER: &str = "doc-symbols: ignore";

/// 整文件免检标记：用于历史快照、设计草案与通用方法论文档。
/// 必须是含该标记的 HTML 注释行（`<!-- doc-symbols: ignore-file … -->`），
/// 散文里提到标记不会生效；命中时会在输出中列出被跳过的文件，避免静默扩大豁免。
const IGNORE_FILE_MARKER: &str = "doc-symbols: ignore-file";

/// 扫描的仓库级文档。
const ROOT_DOCS: &[&str] = &["CLAUDE.md", "README.md", "AGENTS.md"];

/// 扫描的文档目录。
const DOC_DIRS: &[&str] = &["docs", ".claude"];

/// 不参与校验的目录名：第三方 vendored 技能包与构建产物。
const SKIP_DIR_NAMES: &[&str] = &[".system", "target", "node_modules", ".direnv", ".git"];

/// 文件存在性索引只排除构建产物与版本库内部目录：文档指向 vendored 技能包里的真实文件仍然算存在。
const INDEX_SKIP_DIR_NAMES: &[&str] = &["target", "node_modules", ".direnv", ".git"];

/// 参与路径校验的文件扩展名。
const REPO_FILE_EXTENSIONS: &[&str] = &[
    ".rs", ".md", ".toml", ".yml", ".lock", ".nix", ".sh", ".sql", ".json",
];

/// 指向仓库外但属合法的路径前缀：发布流程描述的 nixpkgs 仓库位置。
/// 带 `/` 的引用必须命中这些前缀或真实仓库文件，裸文件名（`self.sql`、`settings.json`）一律不判为文件引用。
const EXTERNAL_PATH_PREFIXES: &[&str] = &["pkgs/", "maintainers/"];

/// 符号索引来源目录。
const SOURCE_DIRS: &[&str] = &["src", "tests"];

fn main() -> ExitCode {
    let root = project_root();
    let known = collect_known_symbols(&root);
    let repo_files = collect_repo_files(&root);
    let mut unknown: Vec<(PathBuf, usize, String)> = Vec::new();
    let mut missing_files: Vec<(PathBuf, usize, String)> = Vec::new();
    let mut skipped: Vec<PathBuf> = Vec::new();

    for doc in iter_docs(&root) {
        if is_ignored_file(&doc) {
            skipped.push(doc);
            continue;
        }
        collect_doc_findings(&doc, &known, &repo_files, &mut unknown, &mut missing_files);
    }

    for doc in &skipped {
        println!(
            "note: skipped {} ({})",
            doc.strip_prefix(&root).unwrap_or(doc).display(),
            IGNORE_FILE_MARKER
        );
    }

    if !unknown.is_empty() {
        eprintln!("Unknown project symbols referenced by docs:");
        for (path, line, symbol) in &unknown {
            eprintln!(
                "  {}:{}: `{}`",
                path.strip_prefix(&root).unwrap_or(path).display(),
                line,
                symbol
            );
        }
    }

    if !missing_files.is_empty() {
        eprintln!("Missing repo files referenced by docs:");
        for (path, line, file) in &missing_files {
            eprintln!(
                "  {}:{}: `{}`",
                path.strip_prefix(&root).unwrap_or(path).display(),
                line,
                file
            );
        }
    }

    if unknown.is_empty() && missing_files.is_empty() {
        println!("OK: docs reference only existing project symbols and files");
        return ExitCode::SUCCESS;
    }

    eprintln!(
        "{} unknown symbol(s), {} missing file reference(s). Fix the reference, or mark \
         the line with `{}`.",
        unknown.len(),
        missing_files.len(),
        IGNORE_MARKER
    );
    ExitCode::FAILURE
}

/// 仓库内所有文件的相对路径，用于把文档中的 `path/to/file.rs` 解析回真实文件。
fn collect_repo_files(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    collect_repo_files_into(root, root, &mut files);
    files
}

fn collect_repo_files_into(root: &Path, dir: &Path, files: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if is_skipped_dir(&path, INDEX_SKIP_DIR_NAMES) || name.starts_with("flake-profile") {
            continue;
        }
        if path.is_dir() {
            collect_repo_files_into(root, &path, files);
        } else if let Ok(relative) = path.strip_prefix(root) {
            files.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// 整文件免检：标记必须单独出现在 HTML 注释行里，避免散文或示例中的提及静默关闭整个文件的检查。
/// 命中时会在输出中列出被跳过的文件，避免静默扩大豁免。
fn is_ignored_file(doc: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(doc) else {
        return false;
    };
    text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("<!--") && trimmed.contains(IGNORE_FILE_MARKER)
    })
}

fn project_root() -> PathBuf {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("no cwd"))
}

/// 不参与符号索引的文件名：门禁自身的测试用例按设计包含不存在的样例符号。
const INDEX_EXCLUDED_FILES: &[&str] = &["check_doc_symbols.rs"];

/// 建立"项目已知符号"集合：`src/`、`tests/` 下出现过的所有标识符与文件名主干。
///
/// 文件名主干让文档可以按名字引用测试文件（`mysql_pool_acceptance` → `tests/mysql_pool_acceptance.rs`）。
/// 索引按行剥离 `//` 注释，避免"只在注释里被提到"的名字被当成存活 API；字符串字面量不剥离，
/// 因此测试 fixture 里的样例名字仍算已知——门禁自身文件因此被排除在索引之外。
///
/// 剥离是文本级的：字符串字面量里的 `//` 会截断该行余下内容，`/* */` 块注释不剥离。
/// 仓库中没有承载标识符的块注释，净影响是双向且极小的，不引入词法分析。
fn collect_known_symbols(root: &Path) -> HashSet<String> {
    let mut known = HashSet::new();
    for dir in SOURCE_DIRS {
        collect_identifiers(&root.join(dir), &mut known);
    }
    known
}

fn collect_identifiers(dir: &Path, known: &mut HashSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_identifiers(&path, known);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let name = entry.file_name().to_string_lossy().to_string();
            if INDEX_EXCLUDED_FILES.contains(&name.as_str()) {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                known.insert(stem.to_string());
            }
            if let Ok(source) = std::fs::read_to_string(&path) {
                for line in source.lines() {
                    let code = line.split("//").next().unwrap_or("");
                    for found in IDENT_RE.find_iter(code) {
                        known.insert(found.as_str().to_string());
                    }
                }
            }
        }
    }
}

/// 收集待校验文档：仓库级文档 + 文档目录下所有 Markdown。
fn iter_docs(root: &Path) -> Vec<PathBuf> {
    let mut docs: Vec<PathBuf> = ROOT_DOCS
        .iter()
        .map(|name| root.join(name))
        .filter(|path| path.exists())
        .collect();

    for dir in DOC_DIRS {
        docs.extend(markdown_files(&root.join(dir)));
    }

    docs
}

fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if is_skipped_dir(&path, SKIP_DIR_NAMES) {
            continue;
        }
        if path.is_dir() {
            files.extend(markdown_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "md") {
            files.push(path);
        }
    }
    files
}

fn is_skipped_dir(path: &Path, skip_names: &[&str]) -> bool {
    path.is_dir()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| skip_names.contains(&name))
}

/// 找出文档中引用了未知项目符号或不存在文件的反引号片段。
fn collect_doc_findings(
    doc: &Path,
    known: &HashSet<String>,
    repo_files: &[String],
    unknown: &mut Vec<(PathBuf, usize, String)>,
    missing_files: &mut Vec<(PathBuf, usize, String)>,
) {
    let Ok(text) = std::fs::read_to_string(doc) else {
        return;
    };

    let mut in_code_fence = false;
    for (index, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence || line.contains(IGNORE_MARKER) {
            continue;
        }

        for span in SPAN_RE.captures_iter(line) {
            let token = &span[1];
            if let Some(path) = candidate_path(token) {
                if !repo_files.iter().any(|file| file.ends_with(&path)) {
                    missing_files.push((doc.to_path_buf(), index + 1, path));
                }
                continue;
            }
            for symbol in candidate_symbols(token) {
                if !known.contains(&symbol) {
                    unknown.push((doc.to_path_buf(), index + 1, symbol));
                }
            }
        }
    }
}

/// 从反引号片段中提取需要校验的符号。
///
/// 返回空表示该片段不是项目符号引用（路径、命令、版本号、外部路径等）。
fn candidate_symbols(token: &str) -> Vec<String> {
    let token = token.trim();
    if token.is_empty() || token.contains(char::is_whitespace) {
        return Vec::new();
    }

    let token = token.strip_suffix("()").unwrap_or(token);
    let segments: Vec<&str> = token.split("::").collect();
    if segments
        .iter()
        .any(|segment| !is_identifier(segment) || segment.is_empty())
    {
        return Vec::new();
    }

    match segments.len() {
        1 => {
            if is_project_symbol_name(token) {
                vec![token.to_string()]
            } else {
                Vec::new()
            }
        }
        _ => {
            // 首段小写 => 外部 crate 或子模块路径，不判为项目符号。
            if !starts_with_uppercase(segments[0]) {
                return Vec::new();
            }
            let mut symbols = vec![segments[0].to_string()];
            if let Some(last) = segments.last()
                && *last != segments[0]
            {
                symbols.push((*last).to_string());
            }
            symbols
        }
    }
}

/// 裸标识符只在形态足够明确时才当作项目符号：蛇形（`show_blocking`）或大驼峰（`DialogStyle`）。
///
/// 单个小写单词（`paths`、`feat`）、camelCase 配置键（`cargoHash`）、全大写常量名
/// （`GITHUB_TOKEN`、`NOT_AVAILABLE`）都会与外部工具、配置键、提交类型混淆，故跳过。
fn is_project_symbol_name(token: &str) -> bool {
    let has_lowercase = token.chars().any(|c| c.is_ascii_lowercase());
    let is_snake = token.contains('_');
    let is_pascal = starts_with_uppercase(token);
    has_lowercase && (is_snake || is_pascal)
}

fn is_identifier(segment: &str) -> bool {
    let mut chars = segment.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// 反引号中的仓库文件引用（`app/runtime/handler.rs`、`src/ui/dock_tabs.rs`）。
///
/// 用后缀匹配解析，因此文档既可以写完整路径，也可以写相对 `src/` 的片段。
/// glob、命令行、目录、URL 一律跳过。
fn candidate_path(token: &str) -> Option<String> {
    let token = token.trim();
    if token.contains(char::is_whitespace) || token.contains("--") {
        return None;
    }
    // glob、花括号展开、占位符、仓库外路径一律跳过。
    if token.contains(['*', '{', '}', '<', '>', '\\'])
        || token.starts_with("http")
        || token.starts_with('/')
        || token.starts_with("~/")
        || token.starts_with("..")
    {
        return None;
    }
    let token = token.strip_prefix("./").unwrap_or(token);
    if !token.contains('/') || EXTERNAL_PATH_PREFIXES.iter().any(|p| token.starts_with(p)) {
        return None;
    }
    // 扩展名必须只出现在结尾，`binding.rs/parser.rs` 这类简写不作为文件引用。
    let extension = REPO_FILE_EXTENSIONS
        .iter()
        .find(|ext| token.ends_with(**ext))?;
    let stem = &token[..token.len() - extension.len()];
    if REPO_FILE_EXTENSIONS.iter().any(|ext| stem.contains(ext)) {
        return None;
    }
    Some(token.to_string())
}

fn starts_with_uppercase(segment: &str) -> bool {
    segment
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_symbols_accepts_project_paths() {
        assert_eq!(
            candidate_symbols("DialogStyle::responsive_widths"),
            vec!["DialogStyle".to_string(), "responsive_widths".to_string()]
        );
        assert_eq!(
            candidate_symbols("show_blocking"),
            vec!["show_blocking".to_string()]
        );
        assert_eq!(
            candidate_symbols("run_frame_with_event()"),
            vec!["run_frame_with_event".to_string()]
        );
    }

    #[test]
    fn candidate_symbols_rejects_external_and_non_symbol_spans() {
        for token in [
            "egui::Order::Foreground",
            "cargo test --workspace --all-features",
            "src/ui/dialogs/common.rs",
            "7.2.0",
            "gridix-driver",
            "keymap.toml",
            "ShowHistory | DeleteTable",
            "Vec<u8>",
        ] {
            assert!(
                candidate_symbols(token).is_empty(),
                "`{token}` 不应被判为项目符号"
            );
        }
    }

    #[test]
    fn candidate_symbols_rejects_bare_names_shared_with_other_domains() {
        for token in [
            "paths",         // 文档/配置键
            "needs",         // nix 属性
            "feat",          // 提交类型
            "cargoHash",     // camelCase 配置键
            "brandColor",    // camelCase 配置键
            "GITHUB_TOKEN",  // 环境变量
            "NOT_AVAILABLE", // 外部枚举值
        ] {
            assert!(
                candidate_symbols(token).is_empty(),
                "`{token}` 不应被判为项目符号"
            );
        }
    }

    /// 历史上真实出现过两处漂移：文档引用了代码中不存在的 API。
    #[test]
    fn candidate_symbols_flag_historical_drift() {
        assert_eq!(
            candidate_symbols("DialogStyle::centered_shortcut"),
            vec!["DialogStyle".to_string(), "centered_shortcut".to_string()]
        );
        assert_eq!(
            candidate_symbols("FooterResult::Dismissed"),
            vec!["FooterResult".to_string(), "Dismissed".to_string()]
        );
    }

    #[test]
    fn collect_doc_findings_skips_fences_and_marked_lines() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let doc = dir.path().join("sample.md");
        std::fs::write(
            &doc,
            "真实符号 `DialogStyle`\n```\n假符号 `Totally::Fabricated`\n```\n\
             豁免行 `Also::Fabricated` <!-- doc-symbols: ignore -->\n未知 `Real::Missing`\n\
             不存在的文件 `src/nope/missing.rs`\n",
        )
        .expect("write sample doc");

        let known: HashSet<String> = ["DialogStyle".to_string()].into_iter().collect();
        let repo_files = vec!["src/ui/dialogs/common.rs".to_string()];
        let mut unknown = Vec::new();
        let mut missing_files = Vec::new();
        collect_doc_findings(&doc, &known, &repo_files, &mut unknown, &mut missing_files);

        let found: Vec<String> = unknown.iter().map(|(_, _, s)| s.clone()).collect();
        assert_eq!(found, vec!["Real".to_string(), "Missing".to_string()]);
        assert_eq!(
            missing_files
                .iter()
                .map(|(_, _, s)| s.clone())
                .collect::<Vec<_>>(),
            vec!["src/nope/missing.rs".to_string()]
        );
    }

    #[test]
    fn ignore_file_marker_requires_an_html_comment_line() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let commented = dir.path().join("commented.md");
        std::fs::write(
            &commented,
            "# Title\n\n<!-- doc-symbols: ignore-file — reason -->\n",
        )
        .expect("write commented doc");
        assert!(is_ignored_file(&commented));

        let prose = dir.path().join("prose.md");
        std::fs::write(
            &prose,
            "# Title\n\nDocuments the `doc-symbols: ignore-file` marker convention.\n",
        )
        .expect("write prose doc");
        assert!(!is_ignored_file(&prose), "散文中提到标记不应让整份文件免检");
    }

    #[test]
    fn candidate_path_accepts_repo_file_references_only() {
        for token in [
            "app/runtime/handler.rs",
            "src/ui/dock_tabs.rs",
            "docs/CHANGELOG.md",
            ".github/workflows/ci.yml",
            "./docs/README.md",
        ] {
            let expected = token.strip_prefix("./").unwrap_or(token);
            assert_eq!(candidate_path(token).as_deref(), Some(expected));
        }

        for token in [
            "src/**/*.rs",
            "cargo test --workspace",
            "src/ui/",
            "https://example.com/a.rs",
            "gridix-driver.rs",
            "~/.omp/agent/validate.sh",
            "../settings.json",
            "binding.rs/parser.rs",
            "input_router/{mod,scopes,actions}.rs",
            "data/query/<backend>.rs",
            "self.sql",
            "settings.json",
            "acceptance.json",
            "keymap.toml",
            "Cargo.toml",
            "pkgs/by-name/gr/gridix/package.nix",
            "maintainers/maintainer-list.nix",
        ] {
            assert!(
                candidate_path(token).is_none(),
                "`{token}` 不应被判为文件引用"
            );
        }
    }
}
