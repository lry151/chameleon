# 便携文件夹布局 + 拒绝指向默认配置目录

整个工具是一个可搬运的文件夹：exe + config.json + 数据根目录（默认 `D:\ChromeTestProfiles\`，界面可改）。导出/导入 = 拷贝 config.json。工具自身单实例锁（防端口与配置写冲突）。

安全边界：任何角色数据目录若指向 Chrome/Edge 的默认配置目录（如 `%LOCALAPPDATA%\Google\Chrome\User Data`），工具拒绝启动并用自然语言提示错误——这是不可协商的安全兜底，绝不覆盖用户的日常浏览器配置。一键关闭也仅作用于测试数据目录。

**Considered Options**: config.json 放 `%APPDATA%`（exe 可装 Program Files，但导出/导入跨目录、便携性差）。