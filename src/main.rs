mod config;

use config::{TriggerType};
use rdev::{listen, simulate, Button, Event, EventType, Key, SimulateError};
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use ctrlc;

// 定义按键类型，可以是键盘按键或鼠标按钮
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionKey {
    Keyboard(Key),
    Mouse(Button),
}

// 定义操作类型：按下或释放
#[derive(Debug, Clone, PartialEq)]
enum ActionType {
    Press,
    Release,
}

// 定义一个操作，包含按键和操作类型
#[derive(Debug, Clone, PartialEq)]
struct Action {
    key: ActionKey,
    action_type: ActionType,
    timestamp: Instant,
}

// 定义连招序列
#[derive(Debug, Clone)]
pub struct Combo {
    name: String,
    sequence: Vec<ActionKey>,
    delays: Vec<(u64, u64)>, // (按下前延迟, 按下后延迟)
    trigger: TriggerType,
    block_original_input: bool,
    active: bool,
}

impl Combo {
    pub fn new(
        name: String, 
        sequence: Vec<ActionKey>, 
        delays: Vec<(u64, u64)>,
        trigger: TriggerType,
        block_original_input: bool
    ) -> Self {
        Combo {
            name,
            sequence,
            delays,
            trigger,
            block_original_input,
            active: true,
        }
    }

    // 执行连招
    fn execute(&self) -> Result<(), SimulateError> {
        println!("执行连招: {}", self.name);
        
        for (idx, key) in self.sequence.iter().enumerate() {
            // 获取当前按键的延迟配置
            let (before_delay, after_delay) = if idx < self.delays.len() {
                self.delays[idx]
            } else {
                (0, 0)
            };
            
            // 执行前延迟
            if before_delay > 0 {
                thread::sleep(Duration::from_millis(before_delay));
            }
            
            // 按下按键
            match key {
                ActionKey::Keyboard(k) => {
                    simulate(&EventType::KeyPress(*k))?;
                    thread::sleep(Duration::from_millis(50)); // 短暂延迟确保按键被识别
                    simulate(&EventType::KeyRelease(*k))?;
                }
                ActionKey::Mouse(b) => {
                    simulate(&EventType::ButtonPress(*b))?;
                    thread::sleep(Duration::from_millis(50)); // 短暂延迟确保按键被识别
                    simulate(&EventType::ButtonRelease(*b))?;
                }
            }
            
            // 执行后延迟
            if after_delay > 0 {
                thread::sleep(Duration::from_millis(after_delay));
            }
        }
        
        Ok(())
    }

    // 检查是否应该屏蔽用户输入
    fn should_block_input(&self, key: Key, key_sequence: &[Key], timeout_ms: u64) -> bool {
        if !self.block_original_input {
            return false;
        }

        match &self.trigger {
            TriggerType::KeySequence { keys, timeout_ms: trigger_timeout } => {
                // 只屏蔽触发键序列中的最后一个按键（如E+R中的R）
                // 并且只有在前面的按键匹配时才屏蔽
                if keys.len() > 1 && keys.last().unwrap() == &key &&
                    key_sequence.len() >= keys.len() - 1 {
                    // 检查序列的前n-1个键是否匹配
                    let start_idx = key_sequence.len() - (keys.len() - 1);
                    let mut matches = true;
                    for (i, &k) in keys.iter().take(keys.len() - 1).enumerate() {
                        if key_sequence[start_idx + i] != k {
                            matches = false;
                            break;
                        }
                    }

                    // 检查序列是否在超时时间内
                    if matches && timeout_ms <= *trigger_timeout {
                        return true;
                    }
                }
            },
            _ => {}
        }
        
        false
    }
}

// 按键历史记录
struct KeyHistory {
    actions: VecDeque<Action>,
    pressed_keys: HashMap<ActionKey, Instant>,
    recent_keys: VecDeque<(Key, Instant)>,
    max_size: usize,
    sequence_max_size: usize,
    history_timeout: Duration,
}

impl KeyHistory {
    fn new(max_size: usize, sequence_max_size: usize, history_timeout_ms: u64) -> Self {
        KeyHistory {
            actions: VecDeque::with_capacity(max_size),
            pressed_keys: HashMap::new(),
            recent_keys: VecDeque::with_capacity(sequence_max_size),
            max_size,
            sequence_max_size,
            history_timeout: Duration::from_millis(history_timeout_ms),
        }
    }

    fn add_action(&mut self, action: Action) {
        // 更新按下的键
        if action.action_type == ActionType::Press {
            self.pressed_keys.insert(action.key.clone(), action.timestamp);
        } else if action.action_type == ActionType::Release {
            self.pressed_keys.remove(&action.key);
        }
        
        // 如果是键盘按键，添加到最近按下的键序列中
        if let ActionKey::Keyboard(key) = &action.key {
            if action.action_type == ActionType::Press {
                self.recent_keys.push_back((*key, action.timestamp));
                
                // 保持最近按键序列不超过最大大小
                if self.recent_keys.len() > self.sequence_max_size {
                    self.recent_keys.pop_front();
                }
            }
        }
        
        // 移除超时的操作
        let now = Instant::now();
        while let Some(front) = self.actions.front() {
            if now.duration_since(front.timestamp) > self.history_timeout {
                self.actions.pop_front();
            } else {
                break;
            }
        }

        // 添加新操作
        self.actions.push_back(action);
        
        // 如果超出最大大小，移除最旧的
        if self.actions.len() > self.max_size {
            self.actions.pop_front();
        }
    }

    // 获取最近按下的按键序列
    fn get_recent_key_sequence(&self) -> Vec<Key> {
        self.recent_keys.iter().map(|(key, _)| *key).collect()
    }

    // 计算最近两个按键之间的时间间隔（毫秒）
    fn get_last_key_interval(&self) -> u64 {
        if self.recent_keys.len() < 2 {
            return 0;
        }
        
        let len = self.recent_keys.len();
        let (_, latest_time) = self.recent_keys[len - 1];
        let (_, prev_time) = self.recent_keys[len - 2];
        
        latest_time.duration_since(prev_time).as_millis() as u64
    }

    // 检查是否匹配按键修饰符组合
    fn matches_key_modifier(&self, modifier: Key, key: Key) -> bool {
        // 检查修饰键是否已按下
        let modifier_pressed = self.pressed_keys.contains_key(&ActionKey::Keyboard(modifier));
        
        // 检查主键是否最近按下
        if self.recent_keys.len() > 0 {
            let (last_key, _) = self.recent_keys.back().unwrap();
            return modifier_pressed && *last_key == key;
        }
        
        false
    }

    // 检查是否匹配一个按键序列
    fn matches_key_sequence(&self, sequence: &[Key], timeout_ms: u64) -> bool {
        if sequence.len() > self.recent_keys.len() {
            return false;
        }
        
        // 获取要匹配的最后N个按键
        let keys = self.get_recent_key_sequence();
        let start = keys.len() - sequence.len();
        
        // 检查是否序列匹配
        for (i, &k) in sequence.iter().enumerate() {
            if keys[start + i] != k {
                return false;
            }
        }
        
        // 检查时间间隔是否在允许范围内
        if self.recent_keys.len() >= sequence.len() {
            let latest_idx = self.recent_keys.len() - 1;
            let earliest_idx = latest_idx - (sequence.len() - 1);
            
            let (_, latest_time) = self.recent_keys[latest_idx];
            let (_, earliest_time) = self.recent_keys[earliest_idx];
            
            let elapsed = latest_time.duration_since(earliest_time).as_millis() as u64;
            return elapsed <= timeout_ms;
        }
        
        false
    }
}

// 应用状态
struct AppState {
    history: KeyHistory,
    combos: Vec<Combo>,
    blocked_keys: HashMap<Key, Instant>,
    current_champion: Option<String>,
}

impl AppState {
    fn new() -> Self {
        // 硬编码的默认设置
        let history_size = 20;
        let history_timeout_ms = 2000;
        
        // 创建按键历史
        let history = KeyHistory::new(
            history_size,
            10, // 按键序列最大长度
            history_timeout_ms,
        );
        
        // 硬编码的连招配置 - 全局连招
        let mut combos = Vec::new();
        
        // 全局连招：按下 Tab 键执行 A -> 25ms -> 鼠标左键
        combos.push(Combo::new(
            "Tab触发A+左键".to_string(),
            vec![
                ActionKey::Keyboard(Key::KeyA),
                ActionKey::Mouse(Button::Left)
            ],
            vec![(0, 25), (0, 0)], // (按下前延迟, 按下后延迟)
            TriggerType::SingleKey(Key::Tab),
            false
        ));
        
        AppState {
            history,
            combos,
            blocked_keys: HashMap::new(),
            current_champion: None,
        }
    }

    // 设置当前英雄并加载相应的连招
    fn set_champion(&mut self, champion_name: String) {
        self.current_champion = Some(champion_name.clone());
        
        // 清除现有的连招
        self.combos.clear();
        
        // 添加全局连招
        self.combos.push(Combo::new(
            "Tab触发A+左键".to_string(),
            vec![
                ActionKey::Keyboard(Key::KeyA),
                ActionKey::Mouse(Button::Left)
            ],
            vec![(0, 25), (0, 0)],
            TriggerType::SingleKey(Key::Tab),
            false
        ));
        
        // 根据英雄添加特定连招
        if champion_name.to_lowercase() == "yasuo" || champion_name.to_lowercase() == "亚索" {
            // 亚索特定连招1: 按下E后150ms内按下R，屏蔽R执行QR
            self.combos.push(Combo::new(
                "亚索E+R触发QR".to_string(),
                vec![
                    ActionKey::Keyboard(Key::KeyQ),
                    ActionKey::Keyboard(Key::KeyR)
                ],
                vec![(0, 0), (0, 0)],
                TriggerType::KeySequence {
                    keys: vec![Key::KeyE, Key::KeyR],
                    timeout_ms: 1500,
                },
                true // 屏蔽原始输入
            ));
            
            // 亚索特定连招2: 按下E后150ms内按下D，屏蔽D执行QD
            self.combos.push(Combo::new(
                "亚索E+D触发QD".to_string(),
                vec![
                    ActionKey::Keyboard(Key::KeyQ),
                    ActionKey::Keyboard(Key::KeyD)
                ],
                vec![(0, 0), (0, 0)],
                TriggerType::KeySequence {
                    keys: vec![Key::KeyE, Key::KeyD],
                    timeout_ms: 1500,
                },
                true // 屏蔽原始输入
            ));
        }
        
        println!("已切换到英雄: {}", self.current_champion.as_ref().unwrap_or(&"无".to_string()));
        
        // 显示加载的连招
        println!("已加载的连招:");
        for (i, combo) in self.combos.iter().enumerate() {
            println!("  {}. {} - 触发条件: {:?}", i + 1, combo.name, combo.trigger);
        }
    }

    // 检查按键是否被屏蔽
    fn is_key_blocked(&mut self, key: Key) -> bool {
        // 清理过期的屏蔽
        let now = Instant::now();
        self.blocked_keys.retain(|_, time| {
            now.duration_since(*time) < Duration::from_millis(100) // 进一步减少屏蔽时间，从200ms改为100ms
        });
        
        self.blocked_keys.contains_key(&key)
    }

    // 屏蔽按键
    fn block_key(&mut self, key: Key) {
        self.blocked_keys.insert(key, Instant::now());
    }

    // 移除按键屏蔽
    fn unblock_key(&mut self, key: Key) {
        self.blocked_keys.remove(&key);
    }

    // 处理按键事件
    fn handle_event(&mut self, event: &Event) -> Option<Event> {
        match event.event_type {
            EventType::KeyPress(key) => {
                println!("Key pressed: {:?}", key);
                
                // 检查按键是否被屏蔽
                if self.is_key_blocked(key) {
                    println!("Key blocked: {:?}", key);
                    return None; // 屏蔽按键
                }
                
                // 创建按键动作
                let action = Action {
                    key: ActionKey::Keyboard(key),
                    action_type: ActionType::Press,
                    timestamp: Instant::now(),
                };
                
                // 更新按键历史
                self.history.add_action(action);
                
                // 检查快捷键 - 切换英雄
                // Shift+Y 切换到亚索
                if key == Key::KeyY && self.history.matches_key_modifier(Key::ShiftLeft, Key::KeyY) {
                    println!("检测到快捷键 Shift+Y，切换到亚索");
                    self.set_champion("亚索".to_string());
                    return Some(event.clone());
                }
                
                // Shift+S 切换到无英雄（全局配置）
                if key == Key::KeyS && self.history.matches_key_modifier(Key::ShiftLeft, Key::KeyS) {
                    println!("检测到快捷键 Shift+S，切换到全局配置");
                    self.current_champion = None;
                    
                    // 恢复默认连招
                    self.combos.clear();
                    self.combos.push(Combo::new(
                        "Tab触发A+左键".to_string(),
                        vec![
                            ActionKey::Keyboard(Key::KeyA),
                            ActionKey::Mouse(Button::Left)
                        ],
                        vec![(0, 25), (0, 0)],
                        TriggerType::SingleKey(Key::Tab),
                        false
                    ));
                    
                    println!("已切换到全局配置");
                    println!("已加载的连招:");
                    for (i, combo) in self.combos.iter().enumerate() {
                        println!("  {}. {} - 触发条件: {:?}", i + 1, combo.name, combo.trigger);
                    }
                    
                    return Some(event.clone());
                }
                
                // 获取最近的按键序列和时间间隔
                let recent_keys = self.history.get_recent_key_sequence();
                let last_interval = self.history.get_last_key_interval();
                
                // 遍历所有连招，检查是否需要触发
                for (idx, combo) in self.combos.iter().enumerate() {
                    if !combo.active {
                        continue;
                    }
                    
                    // 检查是否匹配触发条件
                    let should_trigger = match &combo.trigger {
                        TriggerType::SingleKey(trigger_key) => {
                            *trigger_key == key
                        },
                        TriggerType::KeySequence { keys, timeout_ms } => {
                            if keys.last().unwrap() == &key {
                                self.history.matches_key_sequence(keys, *timeout_ms)
                            } else {
                                false
                            }
                        },
                        TriggerType::KeyModifier { modifier, key: trigger_key } => {
                            *trigger_key == key && self.history.matches_key_modifier(*modifier, key)
                        },
                        TriggerType::Manual => false,
                    };
                    
                    if should_trigger {
                        println!("触发连招: {}", combo.name);
                        
                        // 克隆连招以便在新线程中使用
                        let combo_clone = combo.clone();
                        
                        // 如果需要屏蔽原始输入
                        if combo.block_original_input {
                            // 屏蔽触发键（只屏蔽触发键，不屏蔽连招中的按键）
                            println!("屏蔽触发键: {:?}", key);
                            self.block_key(key);
                            
                            // 在新线程中执行连招
                            thread::spawn(move || {
                                if let Err(e) = combo_clone.execute() {
                                    println!("执行连招失败: {:?}", e);
                                }
                            });
                            
                            // 不传递被屏蔽的按键给游戏
                            return None;
                        } else {
                            // 不屏蔽原始输入的情况下执行连招
                            thread::spawn(move || {
                                if let Err(e) = combo_clone.execute() {
                                    println!("执行连招失败: {:?}", e);
                                }
                            });
                            
                            // 传递原始按键给游戏
                            return Some(event.clone());
                        }
                    }
                }
                
                // 没有触发任何连招，正常传递按键
                Some(event.clone())
            },
            EventType::KeyRelease(key) => {
                println!("Key released: {:?}", key);
                
                // 检查按键是否被屏蔽
                if self.is_key_blocked(key) {
                    println!("Key release blocked: {:?}", key);
                    return None; // 屏蔽按键
                }
                
                let action = Action {
                    key: ActionKey::Keyboard(key),
                    action_type: ActionType::Release,
                    timestamp: Instant::now(),
                };
                
                self.history.add_action(action);
                Some(event.clone())
            },
            EventType::ButtonPress(button) => {
                println!("Mouse button pressed: {:?}", button);
                
                let action = Action {
                    key: ActionKey::Mouse(button),
                    action_type: ActionType::Press,
                    timestamp: Instant::now(),
                };
                
                self.history.add_action(action);
                Some(event.clone())
            },
            EventType::ButtonRelease(button) => {
                println!("Mouse button released: {:?}", button);
                
                let action = Action {
                    key: ActionKey::Mouse(button),
                    action_type: ActionType::Release,
                    timestamp: Instant::now(),
                };
                
                self.history.add_action(action);
                Some(event.clone())
            },
            _ => Some(event.clone()),
        }
    }
}

// 处理命令
fn handle_command(cmd: &str, state: Arc<RwLock<AppState>>) {
    let cmd = cmd.trim();
    
    if cmd.starts_with("champion") {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        if parts.len() == 2 {
            let champion_name = parts[1].to_string();
            let mut state = state.write().unwrap();
            state.set_champion(champion_name);
        } else {
            println!("用法: champion <英雄名称>");
        }
    } else if cmd == "help" {
        println!("可用命令:");
        println!("  champion <英雄名称> - 设置当前英雄");
        println!("  help - 显示帮助");
        println!("  exit/quit - 退出程序");
    } else if cmd == "exit" || cmd == "quit" {
        println!("退出程序...");
        std::process::exit(0);
    } else {
        println!("未知命令: {}", cmd);
        println!("输入 'help' 获取帮助");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("英雄联盟连招脚本 - 增强版");
    
    // 创建一个通道用于优雅退出
    let (exit_tx, exit_rx) = mpsc::channel();
    
    // 创建一个线程处理Ctrl+C信号
    let exit_tx_clone = exit_tx.clone();
    let _ = ctrlc::set_handler(move || {
        println!("\n接收到Ctrl+C，正在退出...");
        let _ = exit_tx_clone.send(());
    });
    
    // 创建应用状态
    let state = Arc::new(RwLock::new(AppState::new()));
    
    // 显示当前英雄和连招
    {
        let state = state.read().unwrap();
        
        if let Some(champion) = &state.current_champion {
            println!("当前英雄: {}", champion);
        } else {
            println!("当前未选择英雄（使用全局脚本）");
        }
        
        println!("已加载的连招:");
        for (i, combo) in state.combos.iter().enumerate() {
            println!("{}. {} - 触发条件: {:?}", i + 1, combo.name, combo.trigger);
        }
    }
    
    println!("按 Ctrl+C 退出程序");
    
    // 创建命令输入线程
    let state_clone = Arc::clone(&state);
    thread::spawn(move || {
        let mut input = String::new();
        loop {
            input.clear();
            print!("> ");
            std::io::stdout().flush().unwrap();
            
            if std::io::stdin().read_line(&mut input).is_ok() {
                if input.trim().is_empty() {
                    continue;
                }
                handle_command(&input, Arc::clone(&state_clone));
            }
        }
    });

    // 在另一个线程中启动键盘监听
    let keyboard_state = Arc::clone(&state);
    thread::spawn(move || {
        // 尝试使用 grab 模式
        let grab_state = Arc::clone(&keyboard_state);
        let grab_result = rdev::grab(move |event| {
            let mut state = grab_state.write().unwrap();
            state.handle_event(&event)
        });
        
        if let Err(error) = grab_result {
            // 如果grab失败，回退到listen模式
            println!("无法使用grab模式，回退到listen模式: {:?}", error);
            println!("注意：在listen模式下，无法屏蔽按键输入");
            
            // 使用另一个克隆用于listen回调
            let listen_state = Arc::clone(&keyboard_state);
            if let Err(error) = listen(move |event| {
                let mut state = listen_state.write().unwrap();
                let _ = state.handle_event(&event);
            }) {
                println!("无法监听键盘事件: {:?}", error);
            }
        }
    });

    // 主线程等待退出信号
    exit_rx.recv()?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_history() {
        let mut history = KeyHistory::new(5, 5, 1000);
        let now = Instant::now();
        
        // 添加一些按键
        history.add_action(Action {
            key: ActionKey::Keyboard(Key::KeyQ),
            action_type: ActionType::Press,
            timestamp: now,
        });
        
        history.add_action(Action {
            key: ActionKey::Keyboard(Key::KeyW),
            action_type: ActionType::Press,
            timestamp: now,
        });
        
        history.add_action(Action {
            key: ActionKey::Keyboard(Key::KeyE),
            action_type: ActionType::Press,
            timestamp: now,
        });
        
        // 测试按键序列
        let recent_keys = history.get_recent_key_sequence();
        assert_eq!(recent_keys.len(), 3);
        assert_eq!(recent_keys[0], Key::KeyQ);
        assert_eq!(recent_keys[1], Key::KeyW);
        assert_eq!(recent_keys[2], Key::KeyE);
    }

    #[test]
    fn test_basic_functionality() {
        // 这只是一个基本的测试，确保CI能够运行测试
        assert_eq!(2 + 2, 4);
    }
}
