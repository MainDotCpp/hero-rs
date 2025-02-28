use std::thread;
use std::time::Duration;
use rdev::{Key, Button, simulate, EventType};

#[cfg(target_os = "windows")]
use winapi::{
    um::winuser::{
        SendInput, INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYEVENTF_KEYUP, MOUSEEVENTF_LEFTDOWN,
        MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, VK_LBUTTON, VK_RBUTTON,
        KEYEVENTF_SCANCODE, MapVirtualKeyW, MAPVK_VK_TO_VSC
    },
    shared::minwindef::UINT,
    shared::windef::POINT,
};

#[cfg(target_os = "windows")]
use std::ptr::null_mut;

// 将 rdev::Key 转换为 Windows VK 码，对于非Windows环境这只是一个占位
pub fn key_to_vk(key: &Key) -> u16 {
    match key {
        Key::KeyA => 0x41,
        Key::KeyB => 0x42,
        Key::KeyC => 0x43,
        Key::KeyD => 0x44,
        Key::KeyE => 0x45,
        Key::KeyF => 0x46,
        Key::KeyG => 0x47,
        Key::KeyH => 0x48,
        Key::KeyI => 0x49,
        Key::KeyJ => 0x4A,
        Key::KeyK => 0x4B,
        Key::KeyL => 0x4C,
        Key::KeyM => 0x4D,
        Key::KeyN => 0x4E,
        Key::KeyO => 0x4F,
        Key::KeyP => 0x50,
        Key::KeyQ => 0x51,
        Key::KeyR => 0x52,
        Key::KeyS => 0x53,
        Key::KeyT => 0x54,
        Key::KeyU => 0x55,
        Key::KeyV => 0x56,
        Key::KeyW => 0x57,
        Key::KeyX => 0x58,
        Key::KeyY => 0x59,
        Key::KeyZ => 0x5A,
        Key::Num0 => 0x30,
        Key::Num1 => 0x31,
        Key::Num2 => 0x32,
        Key::Num3 => 0x33,
        Key::Num4 => 0x34,
        Key::Num5 => 0x35,
        Key::Num6 => 0x36,
        Key::Num7 => 0x37,
        Key::Num8 => 0x38,
        Key::Num9 => 0x39,
        Key::Tab => 0x09,
        Key::Escape => 0x1B,
        Key::ShiftLeft | Key::ShiftRight => 0x10,
        Key::ControlLeft | Key::ControlRight => 0x11,
        Key::Alt | Key::AltGr => 0x12,
        Key::MetaLeft | Key::MetaRight => 0x5B,
        Key::Space => 0x20,
        Key::Return => 0x0D,
        Key::Backspace => 0x08,
        Key::CapsLock => 0x14,
        Key::F1 => 0x70,
        Key::F2 => 0x71,
        Key::F3 => 0x72,
        Key::F4 => 0x73,
        Key::F5 => 0x74,
        Key::F6 => 0x75,
        Key::F7 => 0x76,
        Key::F8 => 0x77,
        Key::F9 => 0x78,
        Key::F10 => 0x79,
        Key::F11 => 0x7A,
        Key::F12 => 0x7B,
        Key::UpArrow => 0x26,
        Key::DownArrow => 0x28,
        Key::LeftArrow => 0x25,
        Key::RightArrow => 0x27,
        _ => 0 // 不支持的键返回0
    }
}

// 添加一个函数来获取虚拟键码对应的扫描码
#[cfg(target_os = "windows")]
fn vk_to_scan_code(vk_code: u16) -> u16 {
    unsafe {
        MapVirtualKeyW(vk_code as u32, MAPVK_VK_TO_VSC) as u16
    }
}

// Windows平台使用SendInput实现
#[cfg(target_os = "windows")]
pub fn send_key_press(key: Key, duration_ms: u64) -> bool {
    let vk_code = key_to_vk(&key);
    if vk_code == 0 {
        println!("[WIN-模拟] 不支持的按键: {:?}", key);
        return false;
    }
    
    // 获取对应的扫描码
    let scan_code = vk_to_scan_code(vk_code);
    
    println!("[WIN-模拟] 底层模拟按下: {:?} (VK: {:#04x}, SC: {:#04x})", key, vk_code, scan_code);
    
    unsafe {
        // 创建按键按下事件
        let mut input = INPUT {
            type_: INPUT_KEYBOARD,
            u: std::mem::zeroed(),
        };
        
        // 同时设置虚拟键码和扫描码
        *input.u.ki_mut() = std::mem::zeroed();
        input.u.ki_mut().wVk = vk_code;
        input.u.ki_mut().wScan = scan_code;
        input.u.ki_mut().dwFlags = KEYEVENTF_SCANCODE; // 使用扫描码标志
        
        // 发送按键按下事件
        let result = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        if result != 1 {
            println!("[WIN-模拟] 按键按下失败: 发送了 {} 个事件，应该发送 1 个", result);
            return false;
        }
        
        // 等待指定时间
        thread::sleep(Duration::from_millis(duration_ms));
        
        // 创建按键释放事件
        input.u.ki_mut().dwFlags = KEYEVENTF_KEYUP | KEYEVENTF_SCANCODE; // 同时使用按键释放和扫描码标志
        
        // 发送按键释放事件
        let result = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        if result != 1 {
            println!("[WIN-模拟] 按键释放失败: 发送了 {} 个事件，应该发送 1 个", result);
            return false;
        }
    }
    
    println!("[WIN-模拟] 底层模拟释放: {:?}", key);
    true
}

// 非Windows平台使用rdev实现
#[cfg(not(target_os = "windows"))]
pub fn send_key_press(key: Key, duration_ms: u64) -> bool {
    println!("[模拟] 底层模拟按下: {:?} (VK: {:#04x})", key, key_to_vk(&key));
    
    // 使用rdev模拟按键按下
    if let Err(e) = simulate(&EventType::KeyPress(key)) {
        println!("[模拟] 按键按下失败: {:?}", e);
        return false;
    }
    
    // 等待指定时间
    thread::sleep(Duration::from_millis(duration_ms));
    
    // 使用rdev模拟按键释放
    if let Err(e) = simulate(&EventType::KeyRelease(key)) {
        println!("[模拟] 按键释放失败: {:?}", e);
        return false;
    }
    
    println!("[模拟] 底层模拟释放: {:?}", key);
    true
}

// Windows平台使用SendInput实现鼠标点击
#[cfg(target_os = "windows")]
pub fn send_mouse_click(button: u32, duration_ms: u64) -> bool {
    unsafe {
        // 创建鼠标事件
        let mut input = INPUT {
            type_: INPUT_MOUSE,
            u: std::mem::zeroed(),
        };
        
        // 根据按钮设置鼠标事件标志
        match button {
            0 => { // 左键
                println!("[WIN-模拟] 底层模拟鼠标左键按下");
                input.u.mi_mut().dwFlags = MOUSEEVENTF_LEFTDOWN;
                
                // 发送鼠标按下事件
                let result = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                if result != 1 {
                    println!("[WIN-模拟] 鼠标左键按下失败");
                    return false;
                }
                
                // 等待指定时间
                thread::sleep(Duration::from_millis(duration_ms));
                
                // 创建鼠标释放事件
                input.u.mi_mut().dwFlags = MOUSEEVENTF_LEFTUP;
                
                // 发送鼠标释放事件
                let result = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                if result != 1 {
                    println!("[WIN-模拟] 鼠标左键释放失败");
                    return false;
                }
                
                println!("[WIN-模拟] 底层模拟鼠标左键释放");
                true
            },
            1 => { // 右键
                println!("[WIN-模拟] 底层模拟鼠标右键按下");
                input.u.mi_mut().dwFlags = MOUSEEVENTF_RIGHTDOWN;
                
                // 发送鼠标按下事件
                let result = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                if result != 1 {
                    println!("[WIN-模拟] 鼠标右键按下失败");
                    return false;
                }
                
                // 等待指定时间
                thread::sleep(Duration::from_millis(duration_ms));
                
                // 创建鼠标释放事件
                input.u.mi_mut().dwFlags = MOUSEEVENTF_RIGHTUP;
                
                // 发送鼠标释放事件
                let result = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                if result != 1 {
                    println!("[WIN-模拟] 鼠标右键释放失败");
                    return false;
                }
                
                println!("[WIN-模拟] 底层模拟鼠标右键释放");
                true
            },
            _ => {
                println!("[WIN-模拟] 不支持的鼠标按钮: {}", button);
                false
            }
        }
    }
}

// 非Windows平台使用rdev实现鼠标点击
#[cfg(not(target_os = "windows"))]
pub fn send_mouse_click(button: u32, duration_ms: u64) -> bool {
    // 鼠标左键
    if button == 0 {
        println!("[模拟] 底层模拟鼠标左键按下");
        
        if let Err(e) = simulate(&EventType::ButtonPress(Button::Left)) {
            println!("[模拟] 鼠标左键按下失败: {:?}", e);
            return false;
        }
        
        // 等待指定时间
        thread::sleep(Duration::from_millis(duration_ms));
        
        if let Err(e) = simulate(&EventType::ButtonRelease(Button::Left)) {
            println!("[模拟] 鼠标左键释放失败: {:?}", e);
            return false;
        }
        
        println!("[模拟] 底层模拟鼠标左键释放");
        return true;
    }
    
    // 鼠标右键
    if button == 1 {
        println!("[模拟] 底层模拟鼠标右键按下");
        
        if let Err(e) = simulate(&EventType::ButtonPress(Button::Right)) {
            println!("[模拟] 鼠标右键按下失败: {:?}", e);
            return false;
        }
        
        // 等待指定时间
        thread::sleep(Duration::from_millis(duration_ms));
        
        if let Err(e) = simulate(&EventType::ButtonRelease(Button::Right)) {
            println!("[模拟] 鼠标右键释放失败: {:?}", e);
            return false;
        }
        
        println!("[模拟] 底层模拟鼠标右键释放");
        return true;
    }
    
    // 不支持的按钮
    println!("[模拟] 不支持的鼠标按钮: {}", button);
    false
}

// 按照序列执行多个按键，并添加随机延迟
pub fn execute_key_sequence(keys: &[Key], min_delay: u64, max_delay: u64) -> bool {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    for key in keys {
        // 随机按键持续时间 (80-120ms)
        let duration = rng.gen_range(80..120);
        
        // 模拟按键
        if !send_key_press(*key, duration) {
            return false;
        }
        
        // 按键之间的随机延迟
        if min_delay < max_delay {
            let delay = rng.gen_range(min_delay..max_delay);
            thread::sleep(Duration::from_millis(delay));
        }
    }
    
    true
} 