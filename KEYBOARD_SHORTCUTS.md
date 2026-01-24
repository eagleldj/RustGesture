# 支持的键盘快捷键格式

本程序支持多种键名格式，以下是所有支持的格式：

## 📝 修饰键

| 格式 | 说明 |
|------|------|
| `VK_CONTROL`, `CONTROL`, `CTRL` | Control 键 |
| `VK_LCONTROL`, `LCONTROL`, `LCTRL` | 左 Control 键 |
| `VK_RCONTROL`, `RCONTROL`, `RCTRL` | 右 Control 键 |
| `VK_SHIFT`, `SHIFT` | Shift 键 |
| `VK_LSHIFT`, `LSHIFT` | 左 Shift 键 |
| `VK_RSHIFT`, `RSHIFT` | 右 Shift 键 |
| `VK_ALT`, `ALT` | Alt 键 |
| `VK_LMENU`, `LALT` | 左 Alt 键 |
| `VK_RMENU`, `RALT` | 右 Alt 键 |
| `VK_LWIN`, `LWIN` | 左 Windows 键 |
| `VK_RWIN`, `RWIN` | 右 Windows 键 |

## 🔤 字母键 (A-Z)

支持两种格式：
- `VK_A`, `VK_B`, ..., `VK_Z`
- `A`, `B`, ..., `Z`

**示例：**
```json
{
  "Keyboard": {
    "keys": ["VK_CONTROL", "VK_L"]
  }
}
```

## 🔢 数字键 (0-9)

支持两种格式：
- `VK_0`, `VK_1`, ..., `VK_9`
- `0`, `1`, ..., `9`

**示例：**
```json
{
  "Keyboard": {
    "keys": ["VK_CONTROL", "VK_1"]
  }
}
```

## ⌨️ 功能键 (F1-F12)

支持两种格式：
- `VK_F1`, `VK_F2`, ..., `VK_F12`
- `F1`, `F2`, ..., `F12`

## 🔧 特殊键

| 格式 | 说明 |
|------|------|
| `VK_BACK`, `BACKSPACE` | 退格键 |
| `VK_TAB`, `TAB` | Tab 键 |
| `VK_RETURN`, `ENTER` | 回车键 |
| `VK_ESCAPE`, `ESC` | Escape 键 |
| `VK_SPACE`, `SPACE` | 空格键 |

## 📋 配置示例

### Windows 锁屏 (Win+L)
```json
"Right → Down": {
  "Keyboard": {
    "keys": ["VK_LWIN", "VK_L"]
  }
}
```

### 新建标签 (Ctrl+T)
```json
"Down → Up": {
  "Keyboard": {
    "keys": ["VK_CONTROL", "VK_T"]
  }
}
```

### 关闭标签 (Ctrl+W)
```json
"Left → Up": {
  "Keyboard": {
    "keys": ["VK_CONTROL", "VK_W"]
  }
}
```

### 刷新 (F5)
```json
"Up → Down": {
  "Keyboard": {
    "keys": ["VK_F5"]
  }
}
```

### 全选 (Ctrl+A)
```json
"Up → Right": {
  "Keyboard": {
    "keys": ["VK_CONTROL", "VK_A"]
  }
}
```

## 💡 提示

1. **推荐使用 `VK_` 前缀格式** - 更清晰明确
2. **键名不区分大小写** - `vk_l` 和 `VK_L` 都可以
3. **组合键顺序** - 修饰键在前，如 `["VK_CONTROL", "VK_C"]`
4. **直接使用字母** - 也可以用 `["CONTROL", "C"]` 这样的格式
