# UI 规格文档

## 1. 窗口属性

| 属性 | 值 |
|-----|-----|
| x:Class | VideoDownloader.MainWindow |
| Title | Classin视频下载工具 |
| Icon | /icon/green-icon.ico |
| Height | 720 |
| Width | 1280 |
| MinHeight | 450 |
| MinWidth | 800 |
| WindowStartupLocation | CenterScreen |
| Background | #F0F2F5 |
| Loaded | Window_Loaded |
| Closing | Window_Closing |
| SizeChanged | Window_SizeChanged |
| UseLayoutRounding | True |
| TextOptions.TextFormattingMode | Display |
| SnapsToDevicePixels | True |

## 2. 颜色资源

| 资源键名 | 色值 |
|---------|------|
| VibrantBlue | #2196F3 |
| VibrantGreen | #4CAF50 |
| VibrantPurple | #9C27B0 |
| VibrantRed | #F44336 |
| VibrantTeal | #00BCD4 |
| VibrantOrange | #FF9800 |
| ParsedColor | Green |
| WaitingColor | Orange |
| DownloadingColor | Orange |
| CompletedColor | Green |
| FailedColor | Red |
| #FFF | White (背景色) |
| #F8F9FA | 浅灰色 (日志区域背景) |
| #4A90E2 | 侧边栏标题蓝色 |
| #D9D9D9 | 分隔线灰色 |
| #333 | 文本深灰色 |
| #E0E0E0 | 列表项分隔线 |

## 3. Converter 及其算法

### 3.1 DarkenColorConverter
- **用途**: 暗化颜色
- **属性**: DarkenFactor (用于不同状态)
- **算法**: 将颜色值按比例暗化
  - DarkenConverter: DarkenFactor="0.3"
  - DarkenConverterPressed: DarkenFactor="0.5"

### 3.2 HeightConverter
- **用途**: 动态计算 ListViewItem 高度
- **算法**: 将窗体实际高度转换为列表项高度

### 3.3 IndexFormatConverter
- **用途**: 格式化序号显示
- **算法**: 将数字格式化为带序号的文本

### 3.4 ScalingConverter
- **用途**: 动态缩放字体大小
- **属性**: BaseWidth="1440"
- **算法**: 根据窗体宽度相对于基准宽度(1440px)的比例，缩放传入的数值参数
  - Formula: result = parameter * (ActualWidth / BaseWidth)
  - 常用参数: 12(状态栏), 13(常规文本), 16(标题), 19(大标题), 22(icon尺寸)

### 3.5 PathMaxWidthConverter
- **用途**: 计算控件宽度的最大值
- **用途**: 动态限制 txtDownloadPath 文本区域宽度
- **算法**: 计算给定元素宽度之间的可用空间

### 3.6 IndexFormatConverter
- **用途**: 格式化序号显示
- **算法**: 将数字转换为带序号的文本格式 (如 "1." 格式)

## 4. Style 定义

### 4.1 Button (默认按钮样式)
- **TargetType**: Button
- **属性**:
  - Foreground: White
  - FontWeight: Medium
  - Padding: 8,6
  - BorderThickness: 0
  - MinWidth: 100
  - MinHeight: 32
  - CornerRadius: 6 (通过ControlTemplate)
- **ControlTemplate**:
  - Border: x:Name="border", Background="{TemplateBinding Background}", CornerRadius="6"
  - ContentPresenter: HorizontalAlignment="Center", VerticalAlignment="Center"
- **Triggers**:
  - IsMouseOver=True: 背景暗化 (DarkenConverter, Factor=0.3)
  - IsPressed=True: 背景暗化 (DarkenConverterPressed, Factor=0.5)

### 4.2 ActionButtonStyle
- **继承**: Button 默认样式
- **Background**: {StaticResource VibrantBlue}
- **MinHeight**: 32

### 4.3 SmallButtonStyle
- **继承**: ActionButtonStyle
- **MinWidth**: 60
- **Padding**: 4,2
- **Margin**: 6,2

### 4.4 CompactActionButtonStyle
- **继承**: ActionButtonStyle
- **Margin**: 3,0
- **Padding**: 2,0
- **MinHeight**: 20
- **VerticalAlignment**: Stretch
- **HorizontalAlignment**: Stretch

### 4.5 CompactSmallButtonStyle
- **继承**: SmallButtonStyle
- **Margin**: 3
- **Padding**: 4,1
- **MinHeight**: 20
- **VerticalAlignment**: Center

### 4.6 TextBlock (默认样式)
- **Margin**: 0,0,0,5
- **Foreground**: #333
- **FontSize**: 动态缩放 (ScalingConverter, Parameter=13)
- **VerticalAlignment**: Center

### 4.7 TitleTextStyle
- **FontWeight**: Bold
- **FontSize**: 动态缩放 (ScalingConverter, Parameter=16)
- **Margin**: 5,0,0,10

### 4.8 LargeTitleTextStyle
- **FontWeight**: Bold
- **FontSize**: 动态缩放 (ScalingConverter, Parameter=19)
- **Margin**: 8,0,0,0

### 4.9 StatusTextStyle
- **VerticalAlignment**: Center
- **Margin**: 0,0,5,0
- **FontSize**: 动态缩放 (ScalingConverter, Parameter=12)

### 4.10 TextBox 样式
- **Padding**: 6
- **Background**: #FFF
- **BorderBrush**: #D9D9D9
- **BorderThickness**: 1
- **FontSize**: 动态缩放 (ScalingConverter, Parameter=13)
- **Margin**: 0,0,0,10

### 4.11 ListView 样式
- **Background**: #FFF
- **BorderBrush**: #D9D9D9
- **BorderThickness**: 1
- **Margin**: 0,0,0,10

### 4.12 StatusBar 样式
- **Background**: #FFF
- **Height**: 动态缩放 (ScalingConverter, Parameter=30)
- **VerticalContentAlignment**: Center

### 4.13 VideoItemStyle (ListViewItem)
- **Padding**: 5
- **Margin**: 0,0,0,2
- **Background**: White
- **BorderBrush**: #E0E0E0
- **BorderThickness**: 0,0,0,1
- **HorizontalContentAlignment**: Stretch
- **VerticalContentAlignment**: Stretch

### 4.14 GridViewColumnHeader 样式
- **HorizontalContentAlignment**: Center
- **Loaded 事件**: GridViewColumnHeader_Loaded