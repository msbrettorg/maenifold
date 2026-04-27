use crate::cli::HistoryArgs;
use crate::error::GateError;
use crate::output::write_line;
use crate::task_repository::TaskRepository;
use crate::task_sections::TaskStageRecord;

#[derive(Debug, Clone)]
pub struct HistoryCommand;

impl HistoryCommand {
    pub fn execute(args: HistoryArgs) -> Result<(), GateError> {
        let task = TaskRepository::find_task(args.context.root.clone(), &args.task_id)?;
        let document = TaskRepository::read_task_document(&args.context.root, &task)?;

        write_line(&format!("{} history", args.task_id))?;
        write_line(&format!("title: {}", document.metadata.title))?;
        write_line(&format!("pipeline: {}", document.metadata.pipeline))?;
        write_line(&format!("location: {}", task.location))?;

        if document.records.is_empty() {
            write_line("")?;
            write_line("No gate history recorded yet.")?;
            return Ok(());
        }

        for (index, record) in document.records.iter().enumerate() {
            write_line("")?;
            write_line(&render_record(index + 1, record))?;
        }

        Ok(())
    }
}

fn render_record(index: usize, record: &TaskStageRecord) -> String {
    let event = match record.outcome.as_deref() {
        Some(outcome) => outcome.to_uppercase(),
        None => record.record_kind.clone(),
    };
    let mut rendered = format!(
        "{index}. {} stage={} event={} agent={}",
        record.recorded_at, record.stage, event, record.agent
    );

    if let Some(model) = &record.model {
        rendered.push_str(&format!("\n   model: {model}"));
    }
    rendered.push_str(&format!("\n   notes: {}", record.notes));
    rendered
}

#[cfg(test)]
mod tests {
    use super::render_record;
    use crate::task_sections::TaskStageRecord;

    #[test]
    fn outcome_records_render_uppercase_event() {
        let record = TaskStageRecord {
            stage: "red-team".to_string(),
            record_kind: "outcome".to_string(),
            outcome: Some("pass".to_string()),
            agent: "red-team".to_string(),
            model: Some("claude-sonnet-4.5".to_string()),
            recorded_at: "2025-04-03T00:00:00Z".to_string(),
            notes: "approved".to_string(),
        };

        let rendered = render_record(2, &record);
        assert!(rendered.contains("event=PASS"));
        assert!(rendered.contains("model: claude-sonnet-4.5"));
        assert!(rendered.contains("notes: approved"));
    }
}
