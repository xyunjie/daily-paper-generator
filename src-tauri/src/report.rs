use crate::config::ensure_config_dir;
use crate::gitlab::CommitInfo;
use crate::jira::TaskInfo;
use docx_rs::*;
use rust_xlsxwriter::*;

pub struct DailyReport {
    pub date: String,
    pub tasks: Vec<TaskInfo>,
    pub commits: Vec<CommitInfo>,
}

pub struct WeeklyWorkItem {
    pub date: String,
    pub contents: Vec<String>,
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

pub fn generate_week_xlsx(
    start_date: &str,
    end_date: &str,
    items: &[WeeklyWorkItem],
    summary: &str,
    key_tasks: &str,
    completion_status: &str,
) -> Result<String, String> {
    ensure_config_dir()?;

    let file_name = format!("周报_{}_{}.xlsx", start_date, end_date);
    let file_path = crate::config::CONFIG_DIR
        .lock()
        .unwrap()
        .join(&file_name);

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // 表头格式：加粗、居中、细边框
    let header_format = Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);

    // 日期格式：居中、细边框、垂直居中
    let date_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);

    // 内容格式：左对齐、细边框、自动换行
    let content_format = Format::new()
        .set_align(FormatAlign::Left)
        .set_align(FormatAlign::VerticalCenter)
        .set_text_wrap()
        .set_border(FormatBorder::Thin);

    // 设置列宽
    worksheet.set_column_width(0, 15).ok();
    worksheet.set_column_width(1, 60).ok();

    // 写入表头
    worksheet.write_string_with_format(0, 0, "日期", &header_format)
        .map_err(|e| format!("写入表头失败: {}", e))?;
    worksheet.write_string_with_format(0, 1, "工作内容", &header_format)
        .map_err(|e| format!("写入表头失败: {}", e))?;

    let mut current_row = 1u32;

    for item in items {
        let row_count = item.contents.len().max(1);
        let start_row = current_row;
        let end_row = current_row + row_count as u32 - 1;

        // 相同日期合并单元格
        if row_count > 1 {
            worksheet
                .merge_range(start_row, 0, end_row, 0, &item.date, &date_format)
                .map_err(|e| format!("合并单元格失败: {}", e))?;
        } else {
            worksheet
                .write_string_with_format(start_row, 0, &item.date, &date_format)
                .map_err(|e| format!("写入日期失败: {}", e))?;
        }

        // 写入工作内容
        if item.contents.is_empty() {
            worksheet
                .write_string_with_format(start_row, 1, "", &content_format)
                .map_err(|e| format!("写入内容失败: {}", e))?;
        } else {
            for (i, content) in item.contents.iter().enumerate() {
                worksheet
                    .write_string_with_format(start_row + i as u32, 1, content, &content_format)
                    .map_err(|e| format!("写入内容失败: {}", e))?;
            }
        }

        current_row += row_count as u32;
    }

    // 写入周总结
    if !summary.is_empty() {
        let summary_label_format = Format::new()
            .set_bold()
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter)
            .set_border(FormatBorder::Thin);

        let summary_content_format = Format::new()
            .set_align(FormatAlign::Left)
            .set_align(FormatAlign::VerticalCenter)
            .set_text_wrap()
            .set_border(FormatBorder::Thin);

        worksheet
            .write_string_with_format(current_row, 0, "本周总结", &summary_label_format)
            .map_err(|e| format!("写入总结标签失败: {}", e))?;
        worksheet
            .write_string_with_format(current_row, 1, summary, &summary_content_format)
            .map_err(|e| format!("写入总结内容失败: {}", e))?;
        let line_count = summary.chars().filter(|&c| c == '\n').count() + 1;
        worksheet.set_row_height(current_row, (line_count as f64 * 15.0).max(40.0)).ok();
        current_row += 1;
    }

    // 写入重点任务
    if !key_tasks.is_empty() {
        let label_format = Format::new()
            .set_bold()
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter)
            .set_border(FormatBorder::Thin);

        let content_fmt = Format::new()
            .set_align(FormatAlign::Left)
            .set_align(FormatAlign::VerticalCenter)
            .set_text_wrap()
            .set_border(FormatBorder::Thin);

        worksheet
            .write_string_with_format(current_row, 0, "重点任务", &label_format)
            .map_err(|e| format!("写入重点任务标签失败: {}", e))?;
        worksheet
            .write_string_with_format(current_row, 1, key_tasks, &content_fmt)
            .map_err(|e| format!("写入重点任务内容失败: {}", e))?;
        let line_count = key_tasks.chars().filter(|&c| c == '\n').count() + 1;
        worksheet.set_row_height(current_row, (line_count as f64 * 15.0).max(40.0)).ok();
        current_row += 1;
    }

    // 写入任务完成情况
    if !completion_status.is_empty() {
        let label_format = Format::new()
            .set_bold()
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter)
            .set_border(FormatBorder::Thin);

        let content_fmt = Format::new()
            .set_align(FormatAlign::Left)
            .set_align(FormatAlign::VerticalCenter)
            .set_text_wrap()
            .set_border(FormatBorder::Thin);

        worksheet
            .write_string_with_format(current_row, 0, "完成情况", &label_format)
            .map_err(|e| format!("写入完成情况标签失败: {}", e))?;
        worksheet
            .write_string_with_format(current_row, 1, completion_status, &content_fmt)
            .map_err(|e| format!("写入完成情况内容失败: {}", e))?;
        let line_count = completion_status.chars().filter(|&c| c == '\n').count() + 1;
        worksheet.set_row_height(current_row, (line_count as f64 * 15.0).max(40.0)).ok();
    }

    workbook
        .save(&file_path)
        .map_err(|e| format!("保存 Excel 文件失败: {}", e))?;

    log::info!("Weekly Excel generated: {:?}", file_path);
    Ok(file_path.to_string_lossy().to_string())
}
