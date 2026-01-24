# 性能监控 - 诊断卡顿问题

## 🔍 新增功能

我添加了详细的性能监控,可以实时测量钩子的执行时间。

### 监控内容

1. **钩子调用计数** - 总共被调用了多少次
2. **最后执行时间** - 最近一次钩子调用的耗时(纳秒级)
3. **自动警告** - 当耗时超过阈值时发出警告

## 📊 性能指标

### 预期性能

| 场景 | 钩子耗时 | 状态 |
|------|---------|------|
| **理想** | < 1μs (1000ns) | ✅ 完美 |
| **良好** | 1-10μs | ✅ 可接受 |
| **警告** | 10-100μs | ⚠️ 需要注意 |
| **危险** | > 100μs | ❌ 会导致卡顿 |

### 输出示例

```
INFO RustGesture: Hook Stats: calls=15234, last_duration=5μs (5123ns)
```

这说明:
- 钩子已被调用 15,234 次
- 最后一次调用耗时 5 微秒 (5123 纳秒)
- **性能良好,不会卡顿** ✅

### 警告示例

```
⚠️  Hook latency elevated: 45μs
⚠️  HOOK LATENCY HIGH: 234μs - This will cause mouse lag!
```

## 🧪 测试步骤

### 1. 运行程序

```bash
cd d:\_src\RustGesture
cargo run
```

### 2. 观察输出

程序会每 **5 秒** 输出一次性能统计:

```
INFO RustGesture v0.1.0 starting...
INFO RustGesture: Windows message loop started
INFO RustGesture: RustGesture started successfully
INFO RustGesture: Gesture recognition is enabled

INFO RustGesture: Hook Stats: calls=0, last_duration=0μs (0ns)
INFO RustGesture: Hook Stats: calls=8234, last_duration=8μs (8234ns)
INFO RustGesture: Hook Stats: calls=16456, last_duration=7μs (7123ns)
...
```

### 3. 移动鼠标测试

- **正常移动** - 看钩子调用次数和耗时
- **按住中键** - 开始追踪,观察耗时变化
- **画手势** - 看性能是否受影响

## 🎯 诊断指南

### 情况 1: 钩子耗时 < 10μs

```
INFO RustGesture: Hook Stats: calls=8234, last_duration=5μs (5123ns)
```

**说明**: 性能优秀 ✅
**预期**: 鼠标应该流畅

如果仍然卡顿 → 问题不在钩子代码,可能是:
- 系统资源问题
- 其他软件冲突
- Windows 消息循环本身的问题

### 情况 2: 钩子耗时 10-100μs

```
⚠️  Hook latency elevated: 45μs
```

**说明**: 性能下降 ⚠️
**可能原因**:
- try_lock 等待
- 处理线程繁忙
- 系统负载高

**解决方案**: 需要进一步优化

### 情况 3: 钩子耗时 > 100μs

```
⚠️  HOOK LATENCY HIGH: 234μs - This will cause mouse lag!
```

**说明**: 严重性能问题 ❌
**必然导致**: 鼠标卡顿

**紧急措施**:
1. 减少处理频率
2. 简化钩子逻辑
3. 检查是否有死锁

## 📈 性能分析

### 正常情况

**闲置时** (不按中键):
```
Hook Stats: calls=5000, last_duration=2μs
```
- 只处理 CallNextHookEx
- 快速返回
- **应该完全流畅**

**追踪时** (按住中键):
```
Hook Stats: calls=5200, last_duration=15μs
```
- 处理更多 MouseMove
- 仍有采样(每5个取1个)
- **应该基本流畅**

### 异常情况

如果看到:
```
Hook Stats: calls=500, last_duration=500μs
```

这说明:
- 钩子调用次数很少(可能被跳过)
- 但单次耗时很长
- **需要检查具体是哪部分代码慢**

## 🔧 调试建议

### 1. 如果卡顿但钩子快

说明问题不在钩子,尝试:
- 检查是否有其他鼠标钩子软件
- 检查系统资源占用
- 尝试在安全模式运行

### 2. 如果钩子慢

立即告诉我具体的性能数据,我会:
- 分析瓶颈
- 优化慢的部分
- 调整采样策略

## 📝 报告格式

请按此格式报告性能数据:

```
启动后正常移动鼠标:
Hook Stats: calls=XXX, last_duration=XXXμs

按住中键后:
Hook Stats: calls=XXX, last_duration=XXXμs

实际感受: [流畅/轻微卡顿/严重卡顿]
```

这样我能准确诊断问题! 🎯
