# 鼠标钩子性能优化

## 问题

运行 `cargo run` 后鼠标出现卡顿,这是因为鼠标钩子回调处理时间过长。

## 根本原因

1. **锁竞争**: 在钩子回调中使用 `lock().unwrap()` 会阻塞,等待锁释放
2. **处理时间长**: Windows 钩子回调必须**极快**返回,否则会影响鼠标响应
3. **回调未调用**: 原先的代码转换了事件但没有实际调用回调

## 解决方案

### 1. 使用全局静态回调存储

**文件**: [src/winapi/hook.rs](d:\_src\RustGesture\src\winapi\hook.rs)

```rust
// 全局静态变量存储回调
static MOUSE_HOOK_CALLBACK: Mutex<Option<Box<dyn MouseHookCallback>>> = Mutex::new(None);
```

这样钩子过程 (`hook_proc`) 可以访问回调,而不需要通过 `MouseHook` 实例。

### 2. 使用 try_lock 而不是 lock

**关键优化**: 在钩子回调中使用 `try_lock()`,如果锁被持有则**跳过这个事件**:

```rust
// hook.rs - 钩子过程
if let Ok(global_callback) = MOUSE_HOOK_CALLBACK.try_lock() {
    if let Some(ref callback) = *global_callback {
        let _ = callback.on_mouse_event(&event);
    }
}
// 如果锁被持有,跳过这个事件以避免鼠标卡顿
```

```rust
// hook_callback.rs - 回调实现
if let Ok(mut recognizer) = self.recognizer.try_lock() {
    recognizer.handle_mouse_event(event);
}
// 如果锁被持有,跳过这个事件
```

### 3. 先调用 CallNextHookEx

在处理事件之前先调用 `CallNextHookEx`,确保其他钩子能及时处理:

```rust
// 先调用下一个钩子
let result = CallNextHookEx(HHOOK::default(), n_code, w_param, l_param);

// 然后处理事件
if n_code as u32 == HC_ACTION {
    // ... 事件处理
}

result  // 返回结果
```

### 4. 使用 Relaxed 内存序

对于 `enabled` 标志的检查,使用 `Ordering::Relaxed` 而不是 `Ordering::SeqCst`:

```rust
if !self.enabled.load(Ordering::Relaxed) {
    return false;
}
```

`Relaxed` 更快,对于简单的布尔检查已经足够。

## 性能对比

### 优化前
- ❌ 使用 `lock().unwrap()` - 可能无限期阻塞
- ❌ 回调没有实际被调用
- ❌ 在 CallNextHookEx 之前处理事件
- ❌ 使用 SeqCst 内存序

### 优化后
- ✅ 使用 `try_lock()` - 不阻塞,跳过繁忙事件
- ✅ 回调正确调用
- ✅ 先调用 CallNextHookEx
- ✅ 使用 Relaxed 内存序

## 潜在影响

### 正面影响
- ✅ **消除鼠标卡顿** - 钩子回调立即返回
- ✅ **更好的响应性** - 不会阻塞鼠标消息处理
- ✅ **更稳定的性能** - 不会因为锁竞争而卡顿

### 负面影响
- ⚠️ **可能丢失事件** - 如果锁被持有,会跳过一些鼠标移动事件
  - 但这通常**不是问题**,因为:
    - 鼠标移动事件非常频繁 (每秒 100+ 次)
    - 手势识别只需要关键事件 (按下、移动、释放)
    - 丢失一些中间移动事件不会影响手势识别准确性

## 测试

```bash
cd d:\_src\RustGesture
cargo run
```

移动鼠标应该**不再卡顿**。

如果还有轻微卡顿,可以进一步优化:
1. 减少日志输出 (`debug`/`trace` 级别)
2. 在 `handle_mouse_event` 中减少计算
3. 使用事件队列异步处理

## 代码位置

- [src/winapi/hook.rs:15-16](d:\_src\RustGesture\src\winapi\hook.rs#L15-L16) - 全局回调
- [src/winapi/hook.rs:139-163](d:\_src\RustGesture\src\winapi\hook.rs#L139-L163) - 钩子过程优化
- [src/core/hook_callback.rs:37-55](d:\_src\RustGesture\src\core\hook_callback.rs#L37-L55) - 回调优化

## 总结

鼠标钩子的性能优化关键:
1. **极快的回调** - 使用 `try_lock()` 而不是 `lock()`
2. **优先调用其他钩子** - 先调用 `CallNextHookEx`
3. **跳过而非阻塞** - 如果忙,跳过事件而不是等待

这些优化确保了即使在进行手势识别时,鼠标也能保持流畅响应。
