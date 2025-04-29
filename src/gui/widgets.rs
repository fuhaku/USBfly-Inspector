// This file contains custom widgets for the USBfly application
use iced::widget::{Container, Row, Text};
use iced::{Element, Length};

// Create a labeled value widget for displaying descriptor fields
pub fn labeled_value<'a, Message>(
    label: &'a str,
    value: &'a str,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
{
    Row::new()
        .spacing(10)
        .align_items(iced::Alignment::Center)
        .width(Length::Fill)
        .push(
            Text::new(label)
                .width(Length::FillPortion(1)),
        )
        .push(
            Text::new(value)
                .width(Length::FillPortion(2)),
        )
        .into()
}

// Create a hex dump widget for raw data display
pub fn hex_dump<'a, Message>(
    data: &[u8],
    bytes_per_row: usize,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
{
    let mut rows = Vec::new();
    
    for chunk in data.chunks(bytes_per_row) {
        let mut row_text = format!("{:04X}: ", rows.len() * bytes_per_row);
        
        // Hex values
        for (i, byte) in chunk.iter().enumerate() {
            row_text.push_str(&format!("{:02X} ", byte));
            
            // Add extra space in the middle for readability
            if i == bytes_per_row / 2 - 1 {
                row_text.push(' ');
            }
        }
        
        // Padding if the row is not full
        for _ in chunk.len()..bytes_per_row {
            row_text.push_str("   ");
        }
        
        // ASCII representation
        row_text.push_str("  ");
        for byte in chunk {
            if byte.is_ascii_graphic() {
                row_text.push(*byte as char);
            } else {
                row_text.push('.');
            }
        }
        
        rows.push(Text::new(row_text));
    }
    
    // Use a Column instead of Container for adding rows
    use iced::widget::Column;
    let column = rows.into_iter().fold(
        Column::new().spacing(2).width(Length::Fill),
        |col, row| col.push(row),
    );
    
    Container::new(column)
        .width(Length::Fill)
        .into()
}
