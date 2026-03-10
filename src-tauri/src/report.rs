use crate::config::ensure_config_dir;
use crate::gitlab::CommitInfo;
use crate::jira::TaskInfo;
use docx_rs::*;

pub struct DailyReport {
    pub date: String,
    pub tasks: Vec<TaskInfo>,
    pub commits: Vec<CommitInfo>,
}

pub fn generate_docx(report: &DailyReport) -> Result<String, String> {
    ensure_config_dir()?;

    let file_name = format!("日报_{}.docx", report.date);
    let file_path = crate::config::CONFIG_DIR
        .lock()
        .unwrap()
        .join(&file_name);

    let mut doc = Docx::new();

    // 标题
    doc = doc.add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text(&format!("工作日报 - {}", report.date)).bold())
            .align(AlignmentType::Center)
            .size(28),
    );

    // 空行
    doc = doc.add_paragraph(Paragraph::new());

    // Jira 任务部分
    doc = doc.add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text("Jira 任务").bold())
            .size(24),
    );

    if report.tasks.is_empty() {
        doc = doc.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text("无更新任务").italic())
        );
    } else {
        for task in &report.tasks {
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text(&format!(
                        "• {} - {} [{}]",
                        task.key, task.summary, task.status
                    )))
            );
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text(&format!("    链接: {}", task.url)).size(18)),
            );
        }
    }

    // GitLab 提交部分
    doc = doc.add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text("GitLab 提交").bold())
            .size(24),
    );

    if report.commits.is_empty() {
        doc = doc.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text("无提交记录").italic())
        );
    } else {
        for commit in &report.commits {
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text(&format!(
                        "• [{}] {} - {}",
                        commit.project_name, commit.short_id, commit.title
                    )))
            );
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text(&format!("    链接: {}", commit.url)).size(18)),
            );
        }
    }

    // 空行
    doc = doc.add_paragraph(Paragraph::new());

    // 统计信息
    doc = doc.add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text(&format!(
                "统计: {} 个任务更新, {} 次提交",
                report.tasks.len(),
                report.commits.len()
            )))
            .italic()
            .size(20),
    );

    // 写入文件
    let file = std::fs::File::create(&file_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    doc.build().pack(file)
        .map_err(|e| format!("Failed to generate docx: {}", e))?;

    log::info!("Report generated: {:?}", file_path);
    Ok(file_path.to_string_lossy().to_string())
}
