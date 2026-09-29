use std::io::{self, Write};

use serde_json::json;

use super::transport::{progress_message, write_message};

pub(super) fn begin(output: &mut impl Write, request_id: &serde_json::Value) -> io::Result<String> {
    let token = format!("actus/request/{}", request_id);
    write_message(
        output,
        &progress_message(
            &token,
            json!({"kind":"begin","title":"Actus semantic query","cancellable":true}),
        ),
    )?;
    Ok(token)
}

pub(super) fn report(output: &mut impl Write, token: &str, message: &str) -> io::Result<()> {
    write_message(output, &progress_message(token, json!({"kind":"report","message":message})))
}

pub(super) fn finish(output: &mut impl Write, token: &str, canceled: bool) -> io::Result<()> {
    let message = if canceled { "canceled" } else { "complete" };
    write_message(output, &progress_message(token, json!({"kind":"end","message":message})))
}
