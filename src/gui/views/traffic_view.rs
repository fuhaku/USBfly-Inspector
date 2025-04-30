use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Command, Element, Length};
use crate::usb::DecodedUSBData;
use crate::usb::USBDescriptor;
use crate::gui::styles;
use crate::gui::styles::color;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::marker::PhantomData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficItem {
    pub timestamp: f64,
    pub raw_data: Vec<u8>,
    pub decoded_data: DecodedUSBData,
    #[serde(skip)]
    _phantom: PhantomData<()>, // Add phantom data to help with sizing
}

// Implement necessary methods for TrafficItem
impl TrafficItem {
    pub fn new(timestamp: f64, raw_data: Vec<u8>, decoded_data: DecodedUSBData) -> Self {
        Self {
            timestamp,
            raw_data,
            decoded_data,
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TreeNodeId(String);

impl TreeNodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    id: TreeNodeId,
    children: Vec<TreeNodeId>,
    expanded: bool,
    data: String,
    item_type: TreeNodeType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreeNodeType {
    Root,
    Device,
    Configuration,
    Interface,
    Endpoint,
    Transaction,
    Setup,
    Data,
    Status,
    Unknown,
}

pub struct TrafficView {
    traffic_data: Vec<TrafficItem>,
    selected_item: Option<usize>,
    filter_text: String,
    auto_scroll: bool,
    tree_nodes: std::collections::HashMap<TreeNodeId, TreeNode>,
    root_nodes: Vec<TreeNodeId>,
    dark_mode: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    ItemSelected(usize),
    FilterChanged(String),
    ToggleAutoScroll(bool),
    ClearTraffic,
    LoadData(Vec<TrafficItem>),
    ToggleTreeNode(TreeNodeId),
    ToggleDarkMode(bool),
    NoOp,
}

impl TrafficView {
    pub fn new() -> Self {
        Self {
            traffic_data: Vec::new(),
            selected_item: None,
            filter_text: String::new(),
            auto_scroll: true,
            tree_nodes: std::collections::HashMap::new(),
            root_nodes: Vec::new(),
            dark_mode: true, // Default to dark mode for hacker-friendly UI
        }
    }
    
    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ItemSelected(index) => {
                self.selected_item = Some(index);
                // When item is selected, build tree nodes for this item
                self.build_tree_for_selected_item();
                Command::none()
            },
            Message::FilterChanged(filter) => {
                self.filter_text = filter;
                Command::none()
            },
            Message::ToggleAutoScroll(auto_scroll) => {
                self.auto_scroll = auto_scroll;
                Command::none()
            },
            Message::ClearTraffic => {
                self.traffic_data.clear();
                self.selected_item = None;
                self.tree_nodes.clear();
                self.root_nodes.clear();
                Command::none()
            },
            Message::LoadData(data) => {
                self.traffic_data = data;
                self.selected_item = if self.traffic_data.is_empty() {
                    None
                } else {
                    Some(0)
                };
                // When data is loaded, build tree for the first item
                self.build_tree_for_selected_item();
                Command::none()
            },
            Message::ToggleTreeNode(node_id) => {
                if let Some(node) = self.tree_nodes.get_mut(&node_id) {
                    node.expanded = !node.expanded;
                }
                Command::none()
            },
            Message::ToggleDarkMode(enabled) => {
                self.dark_mode = enabled;
                Command::none()
            },
            Message::NoOp => Command::none(),
        }
    }
    
    fn build_tree_for_selected_item(&mut self) {
        if let Some(index) = self.selected_item {
            if index < self.traffic_data.len() {
                // Clone the descriptors to avoid borrowing issues
                let descriptors = self.traffic_data[index].decoded_data.descriptors.clone();
                self.build_tree_for_descriptors(&descriptors);
            }
        }
    }
    
    fn build_tree_for_descriptors(&mut self, descriptors: &[USBDescriptor]) {
        // Clear previous tree
        self.tree_nodes.clear();
        self.root_nodes.clear();
        
        // Create root node
        let root_id = TreeNodeId::new("root");
        self.root_nodes.push(root_id.clone());
        
        let root_node = TreeNode {
            id: root_id.clone(),
            children: Vec::new(),
            expanded: true,
            data: "USB Descriptors".to_string(),
            item_type: TreeNodeType::Root,
        };
        
        self.tree_nodes.insert(root_id.clone(), root_node);
        
        // Process descriptors and build hierarchy
        let mut current_device_id = None;
        let mut current_config_id = None;
        let mut current_interface_id = None;
        
        for (i, descriptor) in descriptors.iter().enumerate() {
            match descriptor {
                USBDescriptor::Device(dev) => {
                    let node_id = TreeNodeId::new(format!("device_{}", i));
                    let data = format!("Device Descriptor (VID:{:04X} PID:{:04X})", 
                        dev.vendor_id, dev.product_id);
                    
                    let node = TreeNode {
                        id: node_id.clone(),
                        children: Vec::new(),
                        expanded: true,
                        data,
                        item_type: TreeNodeType::Device,
                    };
                    
                    // Add as child of root
                    if let Some(root) = self.tree_nodes.get_mut(&root_id) {
                        root.children.push(node_id.clone());
                    }
                    
                    self.tree_nodes.insert(node_id.clone(), node);
                    current_device_id = Some(node_id);
                    // Reset lower-level IDs
                    current_config_id = None;
                    current_interface_id = None;
                },
                USBDescriptor::Configuration(cfg) => {
                    let node_id = TreeNodeId::new(format!("config_{}", i));
                    let data = format!("Configuration Descriptor #{}", cfg.configuration_value);
                    
                    let node = TreeNode {
                        id: node_id.clone(),
                        children: Vec::new(),
                        expanded: true,
                        data,
                        item_type: TreeNodeType::Configuration,
                    };
                    
                    // Add as child of device or root
                    if let Some(device_id) = &current_device_id {
                        if let Some(device) = self.tree_nodes.get_mut(device_id) {
                            device.children.push(node_id.clone());
                        }
                    } else if let Some(root) = self.tree_nodes.get_mut(&root_id) {
                        root.children.push(node_id.clone());
                    }
                    
                    self.tree_nodes.insert(node_id.clone(), node);
                    current_config_id = Some(node_id);
                    // Reset lower-level IDs
                    current_interface_id = None;
                },
                USBDescriptor::Interface(iface) => {
                    let node_id = TreeNodeId::new(format!("interface_{}_{}", i, iface.interface_number));
                    let data = format!("Interface #{} (Class: 0x{:02X})", 
                        iface.interface_number, iface.interface_class.get_value());
                    
                    let node = TreeNode {
                        id: node_id.clone(),
                        children: Vec::new(),
                        expanded: true,
                        data,
                        item_type: TreeNodeType::Interface,
                    };
                    
                    // Add as child of configuration or root
                    if let Some(config_id) = &current_config_id {
                        if let Some(config) = self.tree_nodes.get_mut(config_id) {
                            config.children.push(node_id.clone());
                        }
                    } else if let Some(root) = self.tree_nodes.get_mut(&root_id) {
                        root.children.push(node_id.clone());
                    }
                    
                    self.tree_nodes.insert(node_id.clone(), node);
                    current_interface_id = Some(node_id);
                },
                USBDescriptor::Endpoint(ep) => {
                    let node_id = TreeNodeId::new(format!("endpoint_{}_{}", i, ep.endpoint_address));
                    
                    let direction = if ep.endpoint_address & 0x80 == 0x80 {
                        "IN (Device to Host)"
                    } else {
                        "OUT (Host to Device)"
                    };
                    
                    let data = format!("Endpoint 0x{:02X} - {}", ep.endpoint_address, direction);
                    
                    let node = TreeNode {
                        id: node_id.clone(),
                        children: Vec::new(),
                        expanded: true,
                        data,
                        item_type: TreeNodeType::Endpoint,
                    };
                    
                    // Add as child of interface or root
                    if let Some(interface_id) = &current_interface_id {
                        if let Some(interface) = self.tree_nodes.get_mut(interface_id) {
                            interface.children.push(node_id.clone());
                        }
                    } else if let Some(root) = self.tree_nodes.get_mut(&root_id) {
                        root.children.push(node_id.clone());
                    }
                    
                    self.tree_nodes.insert(node_id, node);
                },
                // Handle other descriptor types similarly
                _ => {
                    // Add as a direct child of root for now
                    let node_id = TreeNodeId::new(format!("other_{}", i));
                    let data = format!("{:?}", descriptor);
                    
                    let node = TreeNode {
                        id: node_id.clone(),
                        children: Vec::new(),
                        expanded: true,
                        data,
                        item_type: TreeNodeType::Unknown,
                    };
                    
                    if let Some(root) = self.tree_nodes.get_mut(&root_id) {
                        root.children.push(node_id.clone());
                    }
                    
                    self.tree_nodes.insert(node_id, node);
                }
            }
        }
    }
    
    pub fn add_packet(&mut self, raw_data: Vec<u8>, decoded_data: DecodedUSBData) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
            
        let traffic_item = TrafficItem::new(timestamp, raw_data, decoded_data);
        
        self.traffic_data.push(traffic_item);
        
        if self.auto_scroll {
            self.selected_item = Some(self.traffic_data.len() - 1);
        }
    }
    
    pub fn clear(&mut self) {
        self.traffic_data.clear();
        self.selected_item = None;
    }
    
    pub fn get_traffic_data(&self) -> Option<Vec<TrafficItem>> {
        if self.traffic_data.is_empty() {
            None
        } else {
            Some(self.traffic_data.clone())
        }
    }
    
    // Helper to render a collapsible tree node
    fn render_tree_node(&self, node_id: &TreeNodeId, level: usize) -> Element<Message> {
        if let Some(node) = self.tree_nodes.get(node_id) {
            // Define indentation based on level
            let indent_width = 20.0 * level as f32;
            
            // Create toggle button for expand/collapse
            let toggle_icon = if node.children.is_empty() {
                "   " // No toggle for leaf nodes
            } else if node.expanded {
                "▼ " // Down triangle for expanded
            } else {
                "▶ " // Right triangle for collapsed
            };
            
            // Pick appropriate text color based on node type
            let node_color = if self.dark_mode {
                match node.item_type {
                    TreeNodeType::Root => color::dark::TEXT,
                    TreeNodeType::Device => color::dark::PRIMARY,
                    TreeNodeType::Configuration => color::dark::SECONDARY,
                    TreeNodeType::Interface => color::dark::USB_GREEN,
                    TreeNodeType::Endpoint => color::dark::TEXT,
                    _ => color::dark::TEXT_SECONDARY,
                }
            } else {
                match node.item_type {
                    TreeNodeType::Root => color::TEXT,
                    TreeNodeType::Device => color::PRIMARY,
                    TreeNodeType::Configuration => color::SECONDARY,
                    TreeNodeType::Interface => color::USB_GREEN,
                    TreeNodeType::Endpoint => color::TEXT,
                    _ => color::TEXT_SECONDARY,
                }
            };
            
            // Build the row with toggle button and node content
            let node_row = row![
                // Indentation space
                container(text(""))
                    .width(Length::Fixed(indent_width)),
                
                // Toggle button
                if !node.children.is_empty() {
                    let btn: Element<Message> = button(text(toggle_icon))
                        .on_press(Message::ToggleTreeNode(node_id.clone()))
                        .style(if self.dark_mode {
                            iced::theme::Button::Custom(Box::new(styles::DarkModeTreeNodeButton))
                        } else {
                            iced::theme::Button::Custom(Box::new(styles::TreeNodeButton))
                        })
                        .width(Length::Fixed(30.0))
                        .into();
                    btn
                } else {
                    let container: Element<Message> = container(text(toggle_icon))
                        .width(Length::Fixed(30.0))
                        .into();
                    container
                },
                
                // Node content
                text(&node.data)
                    .style(iced::theme::Text::Color(node_color))
            ]
            .spacing(5)
            .width(Length::Fill);
            
            // Create a container for the node
            let node_container = container(node_row)
                .style(if self.dark_mode {
                    iced::theme::Container::Custom(Box::new(styles::DarkModeTreeNodeContainer))
                } else {
                    iced::theme::Container::Box
                })
                .width(Length::Fill)
                .padding(3);
            
            // If node is expanded, add children
            if node.expanded && !node.children.is_empty() {
                let mut elements = Vec::new();
                elements.push(node_container.into());
                
                // Add all children recursively
                for child_id in &node.children {
                    elements.push(self.render_tree_node(child_id, level + 1));
                }
                
                Column::with_children(elements).into()
            } else {
                // Just return the node without children
                node_container.into()
            }
        } else {
            // Fallback for missing nodes
            container(text("Node not found")).into()
        }
    }
    
    pub fn view(&self) -> Element<Message> {
        // Title with appropriate color based on dark mode
        let title_color = if self.dark_mode {
            color::dark::PRIMARY
        } else {
            color::PRIMARY
        };
        
        let title = text("USB Traffic")
            .size(24)
            .style(iced::theme::Text::Color(title_color));
            
        let filter_label = text("Filter:")
            .style(if self.dark_mode {
                iced::theme::Text::Color(color::dark::TEXT)
            } else {
                iced::theme::Text::Default
            });
        
        let filter_input = text_input("Enter filter", &self.filter_text)
            .on_input(Message::FilterChanged)
            .padding(5)
            .width(Length::FillPortion(3))
            .style(if self.dark_mode {
                iced::theme::TextInput::Custom(Box::new(styles::DarkModeTextInput))
            } else {
                iced::theme::TextInput::Default
            });
            
        // Dark mode toggle button
        let dark_mode_button = if self.dark_mode {
            button("Light Mode")
                .on_press(Message::ToggleDarkMode(false))
                .style(iced::theme::Button::Custom(Box::new(styles::DarkModePrimaryButton)))
        } else {
            button("Dark Mode")
                .on_press(Message::ToggleDarkMode(true))
                .style(iced::theme::Button::Primary)
        };
            
        let auto_scroll_button = if self.auto_scroll {
            button("Auto-scroll: ON")
                .on_press(Message::ToggleAutoScroll(false))
                .style(if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(styles::DarkModePrimaryButton))
                } else {
                    iced::theme::Button::Primary
                })
        } else {
            button("Auto-scroll: OFF")
                .on_press(Message::ToggleAutoScroll(true))
                .style(if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(styles::DarkModeSecondaryButton))
                } else {
                    iced::theme::Button::Secondary
                })
        };
        
        let clear_button = button("Clear")
            .on_press(Message::ClearTraffic)
            .style(iced::theme::Button::Destructive);
            
        let header = row![
            title,
            row![
                filter_label,
                filter_input,
                dark_mode_button,
                auto_scroll_button,
                clear_button
            ]
            .spacing(10)
            .align_items(iced::Alignment::Center)
        ]
        .spacing(20)
        .align_items(iced::Alignment::Center)
        .width(Length::Fill);
        
        // Define traffic_list as Element to handle different return types
        let traffic_list: Element<Message> = if self.traffic_data.is_empty() {
            // Empty state
            container(
                text("No traffic captured yet")
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
                    .vertical_alignment(iced::alignment::Vertical::Center)
                    .style(if self.dark_mode {
                        iced::theme::Text::Color(color::dark::TEXT)
                    } else {
                        iced::theme::Text::Default
                    })
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y()
            .style(if self.dark_mode {
                iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
            } else {
                iced::theme::Container::Box
            })
            .into()
        } else {
            // Filtered list of traffic items
            let filtered_items: Vec<(usize, &TrafficItem)> = self.traffic_data
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    if self.filter_text.is_empty() {
                        return true;
                    }
                    
                    let filter = self.filter_text.to_lowercase();
                    
                    // Filter by hex representation of raw data
                    let hex_data = format!("{:02X?}", item.raw_data);
                    if hex_data.to_lowercase().contains(&filter) {
                        return true;
                    }
                    
                    // Filter by descriptor type or other attributes
                    for descriptor in &item.decoded_data.descriptors {
                        let desc_str = format!("{:?}", descriptor);
                        if desc_str.to_lowercase().contains(&filter) {
                            return true;
                        }
                    }
                    
                    false
                })
                .collect();
            
            let items = Column::with_children(
                filtered_items
                    .iter()
                    .map(|(index, item)| {
                        let formatted_time = format_timestamp(item.timestamp);
                        let data_preview = if item.raw_data.len() > 8 {
                            format!("{:02X?}...", &item.raw_data[..8])
                        } else {
                            format!("{:02X?}", item.raw_data)
                        };
                        
                        let descriptor_type = if let Some(first) = item.decoded_data.descriptors.first() {
                            format!("{:?}", first)
                        } else {
                            "Unknown".to_string()
                        };
                        
                        let row = row![
                            text(formatted_time)
                                .width(Length::FillPortion(1))
                                .style(if self.dark_mode {
                                    iced::theme::Text::Color(color::dark::TEXT)
                                } else {
                                    iced::theme::Text::Default
                                }),
                            text(&data_preview)
                                .width(Length::FillPortion(2))
                                .style(if self.dark_mode {
                                    iced::theme::Text::Color(color::dark::TEXT)
                                } else {
                                    iced::theme::Text::Default
                                }),
                            text(&descriptor_type)
                                .width(Length::FillPortion(3))
                                .style(if self.dark_mode {
                                    iced::theme::Text::Color(color::dark::TEXT)
                                } else {
                                    iced::theme::Text::Default
                                })
                        ]
                        .spacing(10)
                        .padding(5)
                        .width(Length::Fill);
                        
                        if Some(*index) == self.selected_item {
                            container(row)
                                .style(if self.dark_mode {
                                    iced::theme::Container::Custom(Box::new(styles::DarkModeSelectedContainer))
                                } else {
                                    iced::theme::Container::Custom(Box::new(styles::SelectedContainer))
                                })
                                .width(Length::Fill)
                                .into()
                        } else {
                            button(
                                container(row)
                                    .style(if self.dark_mode {
                                        iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                                    } else {
                                        iced::theme::Container::Box
                                    })
                                    .width(Length::Fill)
                            )
                            .width(Length::Fill)
                            .style(if self.dark_mode {
                                iced::theme::Button::Custom(Box::new(styles::DarkModeTreeNodeButton))
                            } else {
                                iced::theme::Button::Text
                            })
                            .on_press(Message::ItemSelected(*index))
                            .into()
                        }
                    })
                    .collect()
            );
            
            scrollable(items)
                .height(Length::Fill)
                .style(if self.dark_mode {
                    iced::theme::Scrollable::Custom(Box::new(styles::DarkModeScrollable))
                } else {
                    iced::theme::Scrollable::Default
                })
                .into()
        };
        
        let selected_item_view = if let Some(index) = self.selected_item {
            if index < self.traffic_data.len() {
                let item = &self.traffic_data[index];
                
                // Format raw data with byte offset display
                let mut hex_data_lines = Vec::new();
                for chunk in item.raw_data.chunks(16) {
                    let offset = hex_data_lines.len() * 16;
                    let hex_values: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
                    let ascii_values: String = chunk.iter()
                        .map(|&b| if b >= 32 && b <= 126 { b as char } else { '.' })
                        .collect();
                    
                    let hex_line = format!("{:04X}: {} | {}", 
                                          offset, 
                                          hex_values.join(" "), 
                                          ascii_values);
                    hex_data_lines.push(hex_line);
                }
                
                let hex_data_view = Column::with_children(
                    hex_data_lines.into_iter()
                        .map(|line| {
                            text(line)
                                .font(iced::Font::MONOSPACE)
                                .style(if self.dark_mode {
                                    iced::theme::Text::Color(color::dark::CODE_GREEN)
                                } else {
                                    iced::theme::Text::Default
                                })
                                .into()
                        })
                        .collect()
                );
                
                // Create a collapsible tree view of descriptors
                let tree_view: Element<Message> = if !self.root_nodes.is_empty() {
                    let mut elements = Vec::new();
                    for root_id in &self.root_nodes {
                        elements.push(self.render_tree_node(root_id, 0));
                    }
                    Column::with_children(elements).into()
                } else {
                    text("No descriptors available")
                        .style(if self.dark_mode {
                            iced::theme::Text::Color(color::dark::TEXT)
                        } else {
                            iced::theme::Text::Default
                        })
                        .into()
                };
                
                // Create a timestamp display
                let timestamp = format_timestamp(item.timestamp);
                
                let header_text_color = if self.dark_mode {
                    color::dark::PRIMARY
                } else {
                    iced::Color::from_rgb(0.0, 0.4, 0.8)
                };
                
                container(
                    column![
                        // Header with timestamp and packet info
                        row![
                            text("Packet Details")
                                .size(18)
                                .width(Length::Fill)
                                .style(if self.dark_mode {
                                    iced::theme::Text::Color(color::dark::TEXT)
                                } else {
                                    iced::theme::Text::Default
                                }),
                            text(format!("Timestamp: {}", timestamp))
                                .size(14)
                                .style(if self.dark_mode {
                                    iced::theme::Text::Color(color::dark::TEXT_SECONDARY)
                                } else {
                                    iced::theme::Text::Default
                                })
                        ],
                        
                        // Tabs for different views (Hex, Descriptors, etc)
                        container(
                            column![
                                text("Raw Hex Data")
                                    .size(16)
                                    .style(iced::theme::Text::Color(header_text_color)),
                                container(hex_data_view)
                                    .style(if self.dark_mode {
                                        iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                                    } else {
                                        iced::theme::Container::Box
                                    })
                                    .padding(10)
                                    .width(Length::Fill)
                            ]
                        )
                        .style(if self.dark_mode {
                            iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                        } else {
                            iced::theme::Container::Box
                        })
                        .padding(10)
                        .width(Length::Fill),
                        
                        // Descriptor tree view
                        container(
                            column![
                                text("USB Descriptors")
                                    .size(16)
                                    .style(iced::theme::Text::Color(header_text_color)),
                                container(tree_view)
                                    .style(if self.dark_mode {
                                        iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                                    } else {
                                        iced::theme::Container::Box
                                    })
                                    .padding(10)
                                    .width(Length::Fill)
                            ]
                        )
                        .style(if self.dark_mode {
                            iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                        } else {
                            iced::theme::Container::Box
                        })
                        .padding(10)
                        .width(Length::Fill),
                    ]
                    .spacing(10)
                    .padding(10)
                )
                .style(if self.dark_mode {
                    iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                } else {
                    iced::theme::Container::Box
                })
                .width(Length::Fill)
            } else {
                container(
                    text("Invalid selection")
                        .style(if self.dark_mode {
                            iced::theme::Text::Color(color::dark::ERROR)
                        } else {
                            iced::theme::Text::Color(color::ERROR)
                        })
                )
                .style(if self.dark_mode {
                    iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                } else {
                    iced::theme::Container::Box
                })
                .width(Length::Fill)
            }
        } else {
            container(
                text("No item selected")
                    .style(if self.dark_mode {
                        iced::theme::Text::Color(color::dark::TEXT_SECONDARY)
                    } else {
                        iced::theme::Text::Color(color::TEXT_SECONDARY)
                    })
            )
            .style(if self.dark_mode {
                iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
            } else {
                iced::theme::Container::Box
            })
            .width(Length::Fill)
        };
        
        // Main content with background color based on dark mode
        let content = column![
            header,
            row![
                container(traffic_list)
                    .style(if self.dark_mode {
                        iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                    } else {
                        iced::theme::Container::Box
                    })
                    .width(Length::FillPortion(2))
                    .height(Length::Fill),
                container(selected_item_view)
                    .width(Length::FillPortion(3))
                    .height(Length::Fill)
            ]
            .spacing(10)
            .height(Length::Fill)
        ]
        .spacing(20)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);
        
        // Apply dark mode background
        if self.dark_mode {
            container(content)
                .style(iced::theme::Container::Custom(Box::new(styles::DarkModeApplicationContainer)))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            content.into()
        }
    }
}

fn format_timestamp(timestamp: f64) -> String {
    let secs = timestamp as u64;
    let nanos = ((timestamp - secs as f64) * 1_000_000_000.0) as u32;
    
    let time = time::OffsetDateTime::from_unix_timestamp(secs as i64)
        .unwrap_or_else(|_| time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc()))
        .replace_nanosecond(nanos)
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    
    time.format(&time::format_description::parse("[hour]:[minute]:[second].[subsecond digits:3]").unwrap())
        .unwrap_or_else(|_| format!("{:.3}", timestamp))
}

// The old build_descriptor_tree function has been replaced by our render_tree_node implementation
