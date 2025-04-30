use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Command, Element, Length};
use crate::usb::DecodedUSBData;
use crate::usb::USBDescriptor;
use crate::gui::styles;
use crate::gui::styles::color;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::marker::PhantomData;
use crate::usb::mitm_traffic::{UsbTransaction, UsbTransferType, UsbDirection};

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
    #[allow(dead_code)]
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
    Transaction, // Transaction node (e.g., control transfer)
    Setup,       // Setup packet
    Data,        // Data packet
    Status,      // Status/ACK packet
    BulkTransfer, // Bulk transfer
    InterruptTransfer, // Interrupt transfer
    IsochronousTransfer, // Isochronous transfer
    ClassRequest, // Class-specific request
    VendorRequest, // Vendor-specific request
    StandardRequest, // Standard request
    Unknown,
    Other,       // Other/Unknown transaction types
}

pub struct TrafficView {
    traffic_data: Vec<TrafficItem>,
    selected_item: Option<usize>,
    filter_text: String,
    auto_scroll: bool,
    tree_nodes: std::collections::HashMap<TreeNodeId, TreeNode>,
    root_nodes: Vec<TreeNodeId>,
    dark_mode: bool,
    capture_active: bool, // Whether traffic capture is currently active
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
            capture_active: false, // Default to capture not active
        }
    }
    
    // Add a method to set the capture active state
    pub fn set_capture_active(&mut self, active: bool) {
        self.capture_active = active;
    }
    
    // Add a method to clear captured traffic
    pub fn clear_traffic(&mut self) {
        self.traffic_data.clear();
        self.selected_item = None;
        self.tree_nodes.clear();
        self.root_nodes.clear();
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
    
    /// Add a high-level USB transaction to the traffic view
    pub fn add_transaction(&mut self, transaction: UsbTransaction) {
        // Create a root node for the transaction
        let transaction_id = format!("tx_{}", transaction.id);
        let transaction_node_id = TreeNodeId::new(transaction_id.clone());
        
        // Determine the transaction type for icon/styling
        let item_type = match transaction.transfer_type {
            UsbTransferType::Control => TreeNodeType::Transaction,
            UsbTransferType::Bulk => TreeNodeType::BulkTransfer,
            UsbTransferType::Interrupt => TreeNodeType::InterruptTransfer,
            UsbTransferType::Isochronous => TreeNodeType::IsochronousTransfer,
            _ => TreeNodeType::Unknown,
        };
        
        // Create a summary for the transaction
        let summary = transaction.get_summary();
        let formatted_time = format!("{:.6}", transaction.timestamp);
        
        // Format the transaction with timestamp
        let data = format!("[{}] {}", formatted_time, summary);
        
        // Create the transaction node
        let mut transaction_node = TreeNode {
            id: transaction_node_id.clone(),
            children: Vec::new(),
            expanded: true,  // Start expanded to show details
            data,
            item_type,
        };
        
        // Add child nodes for each component of the transaction
        let mut child_nodes = Vec::new();
        
        // Setup packet if present
        if let Some(setup) = &transaction.setup_packet {
            let setup_id = format!("{}_setup", transaction_id);
            let setup_node_id = TreeNodeId::new(setup_id);
            
            let mut request_info = format!("bmRequestType: 0x{:02X}, bRequest: 0x{:02X}",
                setup.bmRequestType, setup.bRequest);
                
            if setup.wValue != 0 || setup.wIndex != 0 || setup.wLength != 0 {
                request_info.push_str(&format!(", wValue: 0x{:04X}, wIndex: 0x{:04X}, wLength: {}", 
                    setup.wValue, setup.wIndex, setup.wLength));
            }
            
            let setup_node = TreeNode {
                id: setup_node_id.clone(),
                children: Vec::new(),
                expanded: false,
                data: format!("Setup: {}", setup.request_description),
                item_type: TreeNodeType::Setup,
            };
            
            self.tree_nodes.insert(setup_node_id.clone(), setup_node);
            child_nodes.push(setup_node_id);
            
            // Add details as fields in a separate node if needed
            // This would go here
        }
        
        // Data packet if present
        if let Some(data_packet) = &transaction.data_packet {
            let data_id = format!("{}_data", transaction_id);
            let data_node_id = TreeNodeId::new(data_id);
            
            let direction = match data_packet.direction {
                UsbDirection::HostToDevice => "Host → Device",
                UsbDirection::DeviceToHost => "Device → Host",
                _ => "Unknown Direction",
            };
            
            let data_node = TreeNode {
                id: data_node_id.clone(),
                children: Vec::new(),
                expanded: false,
                data: format!("Data: {} [{}]", data_packet.data_summary, direction),
                item_type: TreeNodeType::Data,
            };
            
            self.tree_nodes.insert(data_node_id.clone(), data_node);
            child_nodes.push(data_node_id);
        }
        
        // Status packet if present
        if let Some(status) = &transaction.status_packet {
            let status_id = format!("{}_status", transaction_id);
            let status_node_id = TreeNodeId::new(status_id);
            
            let status_node = TreeNode {
                id: status_node_id.clone(),
                children: Vec::new(),
                expanded: false,
                data: format!("Status: {}", status.status),
                item_type: TreeNodeType::Status,
            };
            
            self.tree_nodes.insert(status_node_id.clone(), status_node);
            child_nodes.push(status_node_id);
        }
        
        // Add any extra fields as nodes
        if !transaction.fields.is_empty() {
            let fields_id = format!("{}_fields", transaction_id);
            let fields_node_id = TreeNodeId::new(fields_id);
            
            let field_details: Vec<String> = transaction.fields.iter()
                .map(|(key, value)| format!("{}: {}", key, value))
                .collect();
                
            let fields_node = TreeNode {
                id: fields_node_id.clone(),
                children: Vec::new(),
                expanded: false,
                data: format!("Fields: {}", field_details.join(", ")),
                item_type: TreeNodeType::Other,
            };
            
            self.tree_nodes.insert(fields_node_id.clone(), fields_node);
            child_nodes.push(fields_node_id);
        }
        
        // Add children to transaction node
        transaction_node.children = child_nodes;
        
        // Add transaction node to tree
        self.tree_nodes.insert(transaction_node_id.clone(), transaction_node);
        
        // Add to root nodes
        self.root_nodes.push(transaction_node_id);
        
        // Update selection if auto-scroll is enabled
        if self.auto_scroll {
            // This would select the newest transaction in the tree view
            // Implementation depends on how selection is tracked
        }
    }
    
    pub fn clear(&mut self) {
        self.traffic_data.clear();
        self.selected_item = None;
        self.clear_tree_view();
    }
    
    // Clear the tree view representation of traffic
    pub fn clear_tree_view(&mut self) {
        self.tree_nodes.clear();
        self.root_nodes.clear();
        
        // Recreate just the root node for the tree
        let root_id = TreeNodeId::new("root");
        let root_node = TreeNode {
            id: root_id.clone(),
            children: Vec::new(),
            expanded: true,
            data: "USB Traffic".to_string(),
            item_type: TreeNodeType::Root,
        };
        
        self.tree_nodes.insert(root_id, root_node);
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
                    TreeNodeType::Transaction => color::dark::PRIMARY,
                    TreeNodeType::Setup => color::dark::SECONDARY,
                    TreeNodeType::Data => color::dark::USB_GREEN,
                    TreeNodeType::Status => color::dark::USB_YELLOW,
                    TreeNodeType::BulkTransfer => color::dark::PRIMARY,
                    TreeNodeType::InterruptTransfer => color::dark::USB_GREEN,
                    TreeNodeType::IsochronousTransfer => color::dark::USB_CYAN,
                    TreeNodeType::ClassRequest => color::dark::USB_YELLOW,
                    TreeNodeType::VendorRequest => color::dark::USB_CYAN,
                    TreeNodeType::StandardRequest => color::dark::PRIMARY,
                    _ => color::dark::TEXT_SECONDARY,
                }
            } else {
                match node.item_type {
                    TreeNodeType::Root => color::TEXT,
                    TreeNodeType::Device => color::PRIMARY,
                    TreeNodeType::Configuration => color::SECONDARY,
                    TreeNodeType::Interface => color::USB_GREEN,
                    TreeNodeType::Endpoint => color::TEXT,
                    TreeNodeType::Transaction => color::PRIMARY,
                    TreeNodeType::Setup => color::SECONDARY,
                    TreeNodeType::Data => color::USB_GREEN,
                    TreeNodeType::Status => color::USB_YELLOW,
                    TreeNodeType::BulkTransfer => color::PRIMARY,
                    TreeNodeType::InterruptTransfer => color::USB_GREEN,
                    TreeNodeType::IsochronousTransfer => color::USB_CYAN,
                    TreeNodeType::ClassRequest => color::USB_YELLOW,
                    TreeNodeType::VendorRequest => color::USB_CYAN,
                    TreeNodeType::StandardRequest => color::PRIMARY,
                    _ => color::TEXT_SECONDARY,
                }
            };
            
            // Add connection line symbol based on level and position
            let connector_symbol = match (level, node.children.is_empty()) {
                (0, _) => "", // root level, no connector
                (_, true) => "├──►", // non-root leaf node
                (_, false) => "├──┬", // non-root internal node with children
            };
            
            // Build the row with toggle button and node content
            let node_row = row![
                // Indentation space
                container(text(""))
                    .width(Length::Fixed(indent_width)),
                
                // Connector line
                if level > 0 {
                    text(connector_symbol)
                        .style(if self.dark_mode {
                            iced::theme::Text::Color(color::dark::TEXT_SECONDARY)
                        } else {
                            iced::theme::Text::Color(color::TEXT_SECONDARY)
                        })
                } else {
                    text("")
                },
                
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
                    container(text(toggle_icon))
                        .width(Length::Fixed(30.0))
                        .into()
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
            
            // Create a tree structure view for traffic items
            let mut traffic_tree = Column::new().spacing(2);
            
            // Table headers
            let headers = row![
                text("Time")
                    .style(if self.dark_mode {
                        iced::theme::Text::Color(color::dark::PRIMARY)
                    } else {
                        iced::theme::Text::Color(color::PRIMARY)
                    }).width(Length::FillPortion(1)),
                text("Traffic")
                    .style(if self.dark_mode {
                        iced::theme::Text::Color(color::dark::PRIMARY)
                    } else {
                        iced::theme::Text::Color(color::PRIMARY)
                    }).width(Length::FillPortion(4))
            ]
            .padding(5)
            .spacing(10);
            
            traffic_tree = traffic_tree.push(
                container(headers)
                    .style(if self.dark_mode {
                        iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                    } else {
                        iced::theme::Container::Custom(Box::new(styles::HeaderContainer))
                    })
                    .width(Length::Fill)
            );
            
            // Create tree structure for items
            for (index, item) in filtered_items {
                let formatted_time = format_timestamp(item.timestamp);
                
                // Determine item type and make appropriate tree item
                let descriptor_types = item.decoded_data.descriptors.iter()
                    .map(|d| format!("{:?}", d))
                    .collect::<Vec<_>>()
                    .join(", ");
                
                // Create a transaction summary based on the data and descriptors
                let (transaction_summary, node_type) = if descriptor_types.contains("Device") {
                    (create_device_transaction_entry(&item.raw_data, &item.decoded_data), TreeNodeType::Device)
                } else if descriptor_types.contains("Configuration") {
                    (create_config_transaction_entry(&item.raw_data, &item.decoded_data), TreeNodeType::Configuration)
                } else if descriptor_types.contains("Interface") {
                    (create_interface_transaction_entry(&item.raw_data, &item.decoded_data), TreeNodeType::Interface)
                } else if descriptor_types.contains("Endpoint") {
                    (create_endpoint_transaction_entry(&item.raw_data, &item.decoded_data), TreeNodeType::Endpoint)
                } 
                // Identify specific transaction types
                else if item.raw_data.len() >= 8 && item.raw_data[0] == 0x2D {
                    // Control Transfer - SETUP packet (bmRequestType=0x2D)
                    (format!("Control Transfer: SETUP packet, {} bytes", item.raw_data.len()),
                     TreeNodeType::Transaction)
                } 
                // Identify different transfer types
                else if !item.raw_data.is_empty() && (item.raw_data[0] & 0x03) == 0x01 {
                    // Isochronous Transfer (based on PID bits)
                    (format!("Isochronous Transfer on endpoint 0x{:02X}, {} bytes", 
                            item.raw_data[0] & 0x7F, 
                            item.raw_data.len()),
                     TreeNodeType::IsochronousTransfer)
                } 
                else if !item.raw_data.is_empty() && (item.raw_data[0] & 0x03) == 0x03 {
                    // Interrupt Transfer (based on PID bits)
                    (format!("Interrupt Transfer on endpoint 0x{:02X}, {} bytes", 
                            item.raw_data[0] & 0x7F, 
                            item.raw_data.len()),
                     TreeNodeType::InterruptTransfer)
                } 
                else if !item.raw_data.is_empty() && (item.raw_data[0] & 0x03) == 0x02 {
                    // Bulk Transfer (based on PID bits)
                    (format!("Bulk Transfer on endpoint 0x{:02X}, {} bytes", 
                            item.raw_data[0] & 0x7F, 
                            item.raw_data.len()),
                     TreeNodeType::BulkTransfer)
                }
                // Identify data direction 
                else if item.raw_data.len() >= 8 && (item.raw_data[0] & 0x80) != 0 {
                    // Data IN packet (device to host)
                    (format!("IN data packet on endpoint 0x{:02X}, {} bytes", 
                            item.raw_data[0] & 0x7F, 
                            item.raw_data.len()),
                     TreeNodeType::Data)
                } 
                else if item.raw_data.len() <= 4 {
                    // Very small packets are likely status/ACK
                    (format!("Status/ACK packet: {} bytes", item.raw_data.len()),
                     TreeNodeType::Status)
                } 
                else if !item.raw_data.is_empty() && (item.raw_data[0] & 0x80) == 0 {
                    // Generic OUT packet (host to device)
                    (format!("OUT data packet on endpoint 0x{:02X}, {} bytes", 
                            item.raw_data[0] & 0x7F, 
                            item.raw_data.len()),
                     TreeNodeType::Data)
                } 
                // Identify request types
                else if !item.raw_data.is_empty() && (item.raw_data[1] & 0x60) == 0x20 {
                    // Vendor request (based on bmRequestType)
                    (format!("Vendor-specific request, {} bytes", item.raw_data.len()),
                     TreeNodeType::VendorRequest)
                } 
                else if !item.raw_data.is_empty() && (item.raw_data[1] & 0x60) == 0x40 {
                    // Class request (based on bmRequestType)
                    (format!("Class-specific request, {} bytes", item.raw_data.len()),
                     TreeNodeType::ClassRequest)
                } 
                else if !item.raw_data.is_empty() && (item.raw_data[1] & 0x60) == 0x00 {
                    // Standard request (based on bmRequestType)
                    (format!("Standard request, {} bytes", item.raw_data.len()),
                     TreeNodeType::StandardRequest)
                } 
                else if !item.raw_data.is_empty() && (item.raw_data[0] & 0x0F) == 0x0D {
                    // SETUP packet identification (alternate method)
                    (format!("SETUP packet: {} bytes", item.raw_data.len()),
                     TreeNodeType::Setup)
                }
                else {
                    ("Unknown transaction".to_string(), TreeNodeType::Other)
                };
                
                // Display the data with collapsible effect
                let is_selected = Some(index) == self.selected_item;
                let toggle_symbol = "▶"; // Always show as collapsible (we'll handle expansion in the tree view)
                
                let row_content: iced::widget::Row<'_, Message> = row![
                    // Time column
                    text(formatted_time)
                        .width(Length::FillPortion(1))
                        .style(if self.dark_mode {
                            iced::theme::Text::Color(color::dark::TEXT)
                        } else {
                            iced::theme::Text::Default
                        }),
                    
                    // Content column with arrow 
                    row![
                        text(toggle_symbol)
                            .width(Length::Fixed(20.0))
                            .style(if self.dark_mode {
                                iced::theme::Text::Color(color::dark::TEXT_SECONDARY)
                            } else {
                                iced::theme::Text::Color(color::TEXT_SECONDARY)
                            }),
                        text(transaction_summary)
                            .width(Length::Fill)
                            .style(if self.dark_mode {
                                match node_type {
                                    TreeNodeType::Device => iced::theme::Text::Color(color::dark::PRIMARY),
                                    TreeNodeType::Configuration => iced::theme::Text::Color(color::dark::SECONDARY),
                                    TreeNodeType::Interface => iced::theme::Text::Color(color::dark::USB_GREEN),
                                    TreeNodeType::Endpoint => iced::theme::Text::Color(color::dark::USB_YELLOW),
                                    TreeNodeType::Data => iced::theme::Text::Color(color::dark::USB_CYAN),
                                    TreeNodeType::Setup => iced::theme::Text::Color(color::dark::USB_MAGENTA),
                                    TreeNodeType::Status => iced::theme::Text::Color(color::dark::USB_YELLOW),
                                    TreeNodeType::Transaction => iced::theme::Text::Color(color::dark::PRIMARY),
                                    TreeNodeType::BulkTransfer => iced::theme::Text::Color(color::dark::USB_CYAN),
                                    TreeNodeType::InterruptTransfer => iced::theme::Text::Color(color::dark::USB_YELLOW),
                                    TreeNodeType::IsochronousTransfer => iced::theme::Text::Color(color::dark::USB_MAGENTA),
                                    TreeNodeType::ClassRequest => iced::theme::Text::Color(color::dark::USB_MAGENTA),
                                    TreeNodeType::VendorRequest => iced::theme::Text::Color(color::dark::USB_CYAN),
                                    TreeNodeType::StandardRequest => iced::theme::Text::Color(color::dark::PRIMARY),
                                    _ => iced::theme::Text::Color(color::dark::TEXT),
                                }
                            } else {
                                match node_type {
                                    TreeNodeType::Device => iced::theme::Text::Color(color::PRIMARY),
                                    TreeNodeType::Configuration => iced::theme::Text::Color(color::SECONDARY),
                                    TreeNodeType::Interface => iced::theme::Text::Color(color::USB_GREEN),
                                    TreeNodeType::Endpoint => iced::theme::Text::Color(color::USB_YELLOW),
                                    TreeNodeType::Data => iced::theme::Text::Color(color::USB_CYAN),
                                    TreeNodeType::Setup => iced::theme::Text::Color(color::USB_MAGENTA),
                                    TreeNodeType::Status => iced::theme::Text::Color(color::USB_YELLOW),
                                    TreeNodeType::Transaction => iced::theme::Text::Color(color::PRIMARY),
                                    TreeNodeType::BulkTransfer => iced::theme::Text::Color(color::USB_CYAN),
                                    TreeNodeType::InterruptTransfer => iced::theme::Text::Color(color::USB_YELLOW),
                                    TreeNodeType::IsochronousTransfer => iced::theme::Text::Color(color::USB_MAGENTA),
                                    TreeNodeType::ClassRequest => iced::theme::Text::Color(color::USB_MAGENTA),
                                    TreeNodeType::VendorRequest => iced::theme::Text::Color(color::USB_CYAN),
                                    TreeNodeType::StandardRequest => iced::theme::Text::Color(color::PRIMARY),
                                    _ => iced::theme::Text::Color(color::TEXT),
                                }
                            })
                    ]
                    .spacing(5)
                    .width(Length::FillPortion(4))
                ]
                .spacing(10)
                .padding(5)
                .width(Length::Fill);
                
                // Create suitable container based on selection state
                let row_element: Element<Message> = if is_selected {
                    container(row_content)
                        .style(if self.dark_mode {
                            iced::theme::Container::Custom(Box::new(styles::DarkModeSelectedContainer))
                        } else {
                            iced::theme::Container::Custom(Box::new(styles::SelectedContainer))
                        })
                        .width(Length::Fill)
                        .into()
                } else {
                    button(container(row_content)
                        .style(if self.dark_mode {
                            iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                        } else {
                            iced::theme::Container::Box
                        })
                        .width(Length::Fill))
                    .width(Length::Fill)
                    .style(if self.dark_mode {
                        iced::theme::Button::Custom(Box::new(styles::DarkModeTreeNodeButton))
                    } else {
                        iced::theme::Button::Text
                    })
                    .on_press(Message::ItemSelected(index))
                    .into()
                };
                
                traffic_tree = traffic_tree.push(row_element);
                
                // If selected, show child items indented
                if is_selected {
                    // Show sub-transactions
                    // For now, we'll show raw data as separate entries
                    let raw_bytes = create_data_chunks_view(&item.raw_data);
                    for (_i, chunk) in raw_bytes.iter().enumerate() {
                        let child_row: iced::widget::Row<'_, Message> = row![
                            // Time column (empty for child)
                            container(text(""))
                                .width(Length::FillPortion(1)),
                            
                            // Content column with indentation
                            row![
                                container(text(""))
                                    .width(Length::Fixed(20.0)), // Indentation
                                text(format!("├─ {}", chunk))
                                    .font(iced::Font::MONOSPACE)
                                    .style(if self.dark_mode {
                                        iced::theme::Text::Color(color::dark::CODE_GREEN)
                                    } else {
                                        iced::theme::Text::Color(color::CODE_GREEN)
                                    })
                            ]
                            .spacing(5)
                            .width(Length::FillPortion(4))
                        ]
                        .spacing(10)
                        .padding(5)
                        .width(Length::Fill);
                        
                        traffic_tree = traffic_tree.push(
                            container(child_row)
                                .style(if self.dark_mode {
                                    iced::theme::Container::Custom(Box::new(styles::DarkModeChildContainer))
                                } else {
                                    iced::theme::Container::Custom(Box::new(styles::ChildContainer))
                                })
                                .width(Length::Fill)
                        );
                    }
                }
            }
            
            let items = traffic_tree;
            
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

// Create data chunks for display
fn create_data_chunks_view(data: &[u8]) -> Vec<String> {
    let mut result = Vec::new();
    for chunk in data.chunks(16) {
        let offset = result.len() * 16;
        let hex_values: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
        let ascii_values: String = chunk.iter()
            .map(|&b| if b >= 32 && b <= 126 { b as char } else { '.' })
            .collect();
        
        // Format as: 0000: 00 01 02 03 ... | .ABC...
        let mut hex_str = hex_values.join(" ");
        while hex_str.len() < 48 {
            hex_str.push(' ');
        }
        let line = format!("{:04X}: {} | {}", offset, hex_str, ascii_values);
        result.push(line);
    }
    result
}

// Helper to create a device transaction entry
fn create_device_transaction_entry(_raw_data: &[u8], decoded_data: &DecodedUSBData) -> String {
    for descriptor in &decoded_data.descriptors {
        if let USBDescriptor::Device(dev) = descriptor {
            return format!("Device Descriptor: VID:{:04X} PID:{:04X} ({})", 
                dev.vendor_id, 
                dev.product_id,
                get_vendor_name(dev.vendor_id).unwrap_or("Unknown Vendor"));
        }
    }
    "Device Transaction".to_string()
}

// Helper to create a configuration transaction entry
fn create_config_transaction_entry(_raw_data: &[u8], decoded_data: &DecodedUSBData) -> String {
    for descriptor in &decoded_data.descriptors {
        if let USBDescriptor::Configuration(cfg) = descriptor {
            return format!("Configuration Descriptor: Config #{} ({} interfaces, {} mA)", 
                cfg.configuration_value,
                cfg.num_interfaces,
                cfg.max_power * 2);
        }
    }
    "Configuration Transaction".to_string()
}

// Helper to create an interface transaction entry
fn create_interface_transaction_entry(_raw_data: &[u8], decoded_data: &DecodedUSBData) -> String {
    for descriptor in &decoded_data.descriptors {
        if let USBDescriptor::Interface(iface) = descriptor {
            return format!("Interface Descriptor: Interface #{} (Class: {})", 
                iface.interface_number,
                get_class_name_by_value(iface.interface_class.get_value()).unwrap_or("Unknown Class"));
        }
    }
    "Interface Transaction".to_string()
}

// Helper to create an endpoint transaction entry
fn create_endpoint_transaction_entry(_raw_data: &[u8], decoded_data: &DecodedUSBData) -> String {
    for descriptor in &decoded_data.descriptors {
        if let USBDescriptor::Endpoint(ep) = descriptor {
            let direction = if ep.endpoint_address & 0x80 == 0x80 {
                "IN (Device to Host)"
            } else {
                "OUT (Host to Device)"
            };
            
            let endpoint_type = match ep.attributes & 0x03 {
                0 => "Control",
                1 => "Isochronous",
                2 => "Bulk",
                3 => "Interrupt",
                _ => "Unknown"
            };
            
            return format!("Endpoint Descriptor: EP 0x{:02X} {} ({} Transfer, {} bytes/interval)", 
                ep.endpoint_address,
                direction,
                endpoint_type,
                ep.max_packet_size);
        }
    }
    "Endpoint Transaction".to_string()
}

// Helper to get vendor name from vendor ID (simplified version)
fn get_vendor_name(vendor_id: u16) -> Option<&'static str> {
    // Simplified implementation for now
    match vendor_id {
        0x1d50 => Some("Great Scott Gadgets"),
        0x04b4 => Some("Cypress Semiconductor"),
        0x1A86 => Some("QinHeng Electronics"),
        0x0483 => Some("STMicroelectronics"),
        0x2341 => Some("Arduino"),
        0x0403 => Some("FTDI"),
        0x046D => Some("Logitech"),
        0x8087 => Some("Intel"),
        _ => None
    }
}

// Helper to get class name from interface class (simplified version)
fn get_class_name_by_value(class_code: u8) -> Option<&'static str> {
    // Simplified implementation for now
    match class_code {
        0x00 => Some("Interface Association"),
        0x01 => Some("Audio"),
        0x02 => Some("Communications and CDC Control"),
        0x03 => Some("Human Interface Device"),
        0x05 => Some("Physical"),
        0x06 => Some("Image"),
        0x07 => Some("Printer"),
        0x08 => Some("Mass Storage"),
        0x09 => Some("Hub"),
        0x0A => Some("CDC-Data"),
        0x0B => Some("Smart Card"),
        0x0D => Some("Content Security"),
        0x0E => Some("Video"),
        0x0F => Some("Personal Healthcare"),
        0x10 => Some("Audio/Video Devices"),
        0x11 => Some("Billboard Device"),
        0x12 => Some("USB Type-C Bridge"),
        0xDC => Some("Diagnostic Device"),
        0xE0 => Some("Wireless Controller"),
        0xEF => Some("Miscellaneous"),
        0xFE => Some("Application Specific"),
        0xFF => Some("Vendor Specific"),
        _ => None
    }
}

// The old build_descriptor_tree function has been replaced by our render_tree_node implementation
