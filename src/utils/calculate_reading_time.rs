use html2text::from_read;

pub fn calculate_reading_time(html: &str) -> usize {
    // Convert HTML to plain text
    let text = from_read(html.as_bytes(), 80);

    // Count the number of words
    let word_count = text.split_whitespace().count();

    // Calculate the reading time in minutes
    let reading_time = (word_count + 199) / 200;

    reading_time
}
