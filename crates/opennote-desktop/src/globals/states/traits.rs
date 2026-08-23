use gpui::SharedString;

pub trait SelectedText {
    /// Get and remove the selected text from the state
    fn pop_selected_text(&mut self) -> Option<SharedString>;

    /// Set selected text to the state
    fn set_selected_text(&mut self, text: impl Into<SharedString>);
}
