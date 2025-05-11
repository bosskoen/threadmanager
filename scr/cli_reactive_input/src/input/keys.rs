pub enum Key {
    Char(char),      // Normal printable character
    Enter,           // Enter/Return key            0x0D   // 0x0A
    Tab,             // Tab key                     0x09        
    Backspace,       // Backspace key               0x7F   // 0x08 
    Escape,          // Escape key (alone)          0x1B

    Delete,          // Delete key     esc [ 3 ~   // 0x1B 0x5B 0x33 0x7E
    Home,            // Home key       eSC [ H or ESC O H  or esc [ 1 ~  // 0x1B 0x5B 0x48 or 0x1B 0x4F 0x48
    End,             // End key        eSC [ F or ESC O F  or esc [ 4 ~  // 0x1B 0x5B 0x46 or 0x1B 0x4F 0x46

    ArrowUp,        // Up arrow key   eSC [ A or ESC O A   // 0x1B 0x5B 0x41 or 0x1B 0x4F 0x41
    ArrowDown,      // Down arrow key eSC [ B or ESC O B   // 0x1B 0x5B 0x42 or 0x1B 0x4F 0x42
    ArrowLeft,      // Left arrow key eSC [ D or ESC O D   // 0x1B 0x5B 0x44 or 0x1B 0x4F 0x44
    ArrowRight,     // Right arrow key eSC [ C or ESC O C   // 0x1B 0x5B 0x43 or 0x1B 0x4F 0x43

    CtrlC,           // Ctrl+C (common interrupt)       0x03    
    CtrlD,           // Ctrl+D (EOF on Unix)            0x04

    Unknown,         // Anything unrecognized

    // Function keys
}