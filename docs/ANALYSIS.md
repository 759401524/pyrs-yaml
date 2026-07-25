# Rust YAML 库对比分析

## 现有 Rust YAML 库

### 1. yaml-rust2
- **仓库**: https://github.com/Ethiraric/yaml-rust2
- **状态**: 维护模式（仅基本维护）
- **特性**: YAML 1.2 合规，纯 Rust
- **API**: `YamlLoader`, `YamlEmitter`
- **优势**: 成熟稳定，API 简单
- **劣势**: 不保留注释，维护模式

### 2. saphyr
- **仓库**: https://github.com/saphyr-rs/saphyr
- **状态**: 活跃维护
- **特性**: YAML 1.2 完全合规，Event-based API
- **API**: `Yaml`, `YamlEmitter`, `Parser`
- **优势**: 完全合规，活跃维护，Event API
- **劣势**: 高层 API 会丢失注释

### 3. serde_yaml (已废弃)
- **仓库**: https://github.com/dtolnay/serde-yaml
- **状态**: 已废弃，不再维护
- **特性**: Serde 集成
- **劣势**: 不再维护，丢失注释

## pyamlium_custom 定位

### 核心优势
1. **Round-Trip 支持**: 100% 保留注释、锚点、标签、chomping
2. **自定义 AST**: 可扩展的 AST 结构
3. **Python 绑定**: PyO3 直接调用
4. **YAML 1.2 合规**: 98.1% YAML Test Suite 通过率
5. **高性能**: Rust 后端

### 与现有库的差异

| 特性 | yaml-rust2 | saphyr | serde_yaml | pyamlium_custom |
|------|------------|--------|------------|-----------------|
| YAML 1.2 | ✅ | ✅ | ❌ | ✅ |
| Round-Trip | ❌ | ❌ | ❌ | ✅ |
| 注释保留 | ❌ | ❌ | ❌ | ✅ |
| Python 绑定 | ❌ | ❌ | ❌ | ✅ |
| 活跃维护 | ⚠️ | ✅ | ❌ | ✅ |
| 自定义 AST | ❌ | ❌ | ❌ | ✅ |

## 是否需要独立项目？

### 支持独立项目的理由
1. **独特价值**: Round-Trip 支持是现有库没有的
2. **Python 生态**: 直接提供 Python 绑定
3. **可扩展性**: 自定义 AST 可以添加业务特定功能
4. **性能**: Rust 后端比纯 Python 快 25x

### 反对独立项目的理由
1. **维护成本**: 需要持续维护和更新
2. **功能重叠**: 基础 YAML 解析功能与 saphyr 重叠
3. **社区规模**: 小众库，用户群体有限

### 建议

**保持独立项目**，但明确定位：

1. **不是通用 YAML 库** - saphyr 已经做得很好
2. **而是 Python YAML 工具** - 专注于 Python 生态
3. **核心价值**: Round-Trip + 高性能 + Python 绑定

### 未来发展方向

1. **扩展 Python API**: 添加更多 Python 友好的 API
2. **性能优化**: 继续优化解析和序列化速度
3. **功能增强**: 支持更多 YAML 1.2 特性
4. **文档完善**: 提供更好的使用文档和示例
