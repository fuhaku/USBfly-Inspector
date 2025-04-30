use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref VENDOR_MAP: HashMap<u16, &'static str> = {
        let mut m = HashMap::new();
        
        // USB-IF assigned vendor IDs (partial list - common vendors)
        m.insert(0x0001, "Apple, Inc.");
        m.insert(0x0409, "NEC Corporation");
        m.insert(0x045E, "Microsoft Corporation");
        m.insert(0x046D, "Logitech, Inc.");
        m.insert(0x04B0, "Nikon Corporation");
        m.insert(0x04B8, "Seiko Epson Corp.");
        m.insert(0x04D8, "Microchip Technology, Inc.");
        m.insert(0x04E8, "Samsung Electronics Co., Ltd.");
        m.insert(0x04F2, "Chicony Electronics Co., Ltd.");
        m.insert(0x0502, "Acer, Inc.");
        m.insert(0x0557, "ATEN International Co., Ltd.");
        m.insert(0x056A, "Wacom Co., Ltd.");
        m.insert(0x057C, "AVM GmbH");
        m.insert(0x05AC, "Apple, Inc.");
        m.insert(0x0644, "TEAC Corporation");
        m.insert(0x065A, "SiGma Micro");
        m.insert(0x067B, "Prolific Technology, Inc.");
        m.insert(0x06CB, "Synaptics, Inc.");
        m.insert(0x07D1, "D-Link Corporation");
        m.insert(0x08BB, "Texas Instruments, Inc.");
        m.insert(0x0930, "Toshiba Corporation");
        m.insert(0x093A, "Pixart Imaging, Inc.");
        m.insert(0x0951, "Kingston Technology");
        m.insert(0x09DA, "A4Tech Co., Ltd.");
        m.insert(0x0A5C, "Broadcom Corporation");
        m.insert(0x0A81, "Chesen Electronics Corp.");
        m.insert(0x0B05, "ASUSTek Computer, Inc.");
        m.insert(0x0BC2, "Seagate Technology LLC");
        m.insert(0x0C45, "Microdia");
        m.insert(0x0CF3, "Qualcomm Atheros Communications");
        m.insert(0x0D8C, "C-Media Electronics, Inc.");
        m.insert(0x0DB0, "Micro Star International");
        m.insert(0x0E0F, "VMware, Inc.");
        m.insert(0x0FCA, "Research In Motion, Ltd.");
        m.insert(0x1004, "LG Electronics, Inc.");
        m.insert(0x1022, "Advanced Micro Devices, Inc.");
        m.insert(0x103C, "Hewlett-Packard Company");
        m.insert(0x10C4, "Silicon Labs");
        m.insert(0x12D1, "Huawei Technologies Co., Ltd.");
        m.insert(0x1307, "USBest Technology");
        m.insert(0x13D3, "IMC Networks");
        m.insert(0x13FE, "Kingston Technology Company");
        m.insert(0x1410, "Novatel Wireless");
        m.insert(0x152D, "JMicron Technology Corp.");
        m.insert(0x154B, "PNY");
        m.insert(0x17EF, "Lenovo");
        m.insert(0x1871, "Avision");
        m.insert(0x18D1, "Google Inc.");
        m.insert(0x192F, "Sunrise Telecom");
        m.insert(0x19D2, "ZTE WCDMA Technologies MSM");
        m.insert(0x1A40, "Terminus Technology, Inc.");
        m.insert(0x1B1C, "Corsair");
        m.insert(0x1D6B, "Linux Foundation");
        m.insert(0x1D50, "Great Scott Gadgets"); // Cynthion / Great Scott Gadgets
        m.insert(0x2001, "D-Link Corporation");
        m.insert(0x2109, "VIA Labs, Inc.");
        m.insert(0x2222, "MacAlly");
        m.insert(0x2232, "Silicon Motion");
        m.insert(0x2537, "Norelsys");
        m.insert(0x4348, "WinChipHead");
        m.insert(0x8086, "Intel Corporation");
        
        m
    };
}

#[allow(dead_code)]
pub fn lookup_vendor(vendor_id: u16) -> Option<String> {
    VENDOR_MAP.get(&vendor_id).map(|s| s.to_string())
}

#[allow(dead_code)]
pub fn get_vendor_ids() -> Vec<(u16, String)> {
    VENDOR_MAP
        .iter()
        .map(|(&id, &name)| (id, name.to_string()))
        .collect()
}
