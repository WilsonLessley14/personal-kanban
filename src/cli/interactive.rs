use anyhow::{anyhow, Result};
use dialoguer::{Confirm, Input, Select};

/// Prompt for text input with a label.
/// Returns `Err` if stdin is not a TTY (caller should handle non-interactive error).
pub fn prompt_input<T: std::fmt::Display>(prompt: &str, default: Option<&str>) -> Result<String> {
    let mut builder = Input::<String>::new().with_prompt(prompt);
    if let Some(d) = default {
        builder = builder.default(d.to_string());
    }
    builder.interact_text().map_err(Into::into)
}

/// Prompt for a selection from a list.
/// Returns the selected index.
pub fn prompt_select(items: &[&str], prompt: &str) -> Result<usize> {
    let selection = Select::new().with_prompt(prompt).items(items).interact()?;
    Ok(selection)
}

/// Prompt for a yes/no confirmation.
/// Returns `true` if the user confirms.
pub fn prompt_confirm(prompt: &str) -> Result<bool> {
    let confirmed = Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()?;
    Ok(confirmed)
}

/// Check if stdin is a TTY (interactive).
pub fn is_interactive() -> bool {
    atty::is(atty::Stream::Stdin)
}

/// Prompt for a required value. If already provided in `value`, return it.
/// If interactive (is_tty), prompt the user. If not interactive, return an error.
/// `choices` provides a selection list if non-empty; otherwise free-form input.
pub fn prompt_required(
    value: Option<String>,
    label: &str,
    is_tty: bool,
    choices: &[String],
) -> Result<String> {
    if let Some(v) = value {
        return Ok(v);
    }

    if !is_tty {
        return Err(anyhow!(
            "missing required argument: '{}'. Provide it or run interactively.",
            label.to_lowercase()
        ));
    }

    if !choices.is_empty() {
        let selection = Select::new()
            .with_prompt(label)
            .items(choices)
            .default(0)
            .interact()
            .map_err(|e| anyhow!("{}", e))?;
        Ok(choices[selection].clone())
    } else {
        let input: String = Input::new()
            .with_prompt(label)
            .interact_text()
            .map_err(|e| anyhow!("{}", e))?;
        if input.trim().is_empty() {
            return Err(anyhow!("{} cannot be empty", label));
        }
        Ok(input.trim().to_string())
    }
}

/// Show a confirmation summary for task addition and ask for confirmation.
pub fn confirm_task_add(
    title: &str,
    column: &str,
    desc: &str,
    priority: &str,
    is_tty: bool,
) -> Result<bool> {
    if !is_tty {
        return Ok(true);
    }

    println!("Task summary:");
    println!("  Title:    {}", title);
    println!("  Column:   {}", column);
    if !desc.is_empty() {
        println!("  Desc:     {}", desc);
    }
    println!("  Priority: {}", priority);

    Confirm::new()
        .with_prompt("Add this task?")
        .default(true)
        .interact()
        .map_err(|e| anyhow!("{}", e))
}
