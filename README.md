# 今天是周五吗

[bilibili视频介绍](https://www.bilibili.com/video/BV1gf8TerEiX)
[MakerWorld地址](https://makerworld.com.cn/zh/models/375640)

Top | Bottom
-|-
<img src="./Image/frontview.jpg" width=320 title="时间同步模式的显示内容"/> | <img src="./Image/backview.png" width=320 title="时间同步模式的显示内容"/>

这是一个使用CH582F作为主控, 电子墨水屏为显示载体, 由一颗CR2032纽扣电池驱动的装置.

其本质上是一个日历, 但是加入了周五的判断, 使其成为一个周五检测器.

`受即刻App的iOS小部件启发而来`


## PCB 预览
Top | Bottom
-|-
<img src="./Image/geber_bottom.png" width=320 title="时间同步模式的显示内容"/> | <img src="./Image/geber_top.png" width=320 title="时间同步模式的显示内容"/>


## 基础参数

### 硬件参数
* 主控 CH582F 32KB Ram + 448KB Flash
* 时钟芯片 PCF8563T
* 屏幕 1.54英寸墨水屏 SSD1607
* DCDC芯片 SGM6603-3.3YN6G
* 电源 CR2032电池

#### 注: 
* 当使用屏幕驱动IC为SSD1607时候, **需要焊接SGM6603**, 因为需要借助SGM6603来彻底断开墨水屏供电
    * 这块屏幕是我从闲鱼上捡来的, 买回来发现和合宙9.9块钱的墨水屏丝印一样, 驱动IC也一样.
    * 在实际使用时候,休眠状态下功耗始终降低不下来. 于是我一怒之下怒一下了, 加了个DCDC来管理墨水屏的供电. 有效的将他的休眠电流从70μA降低到了3μA
* 当使用屏幕驱动IC为SSD1681时候, **不需要焊接SGM6603**, 此时使用0欧电阻短接SGM6603的Pin5和Pin6, 并且无需焊接SGM6603下方4.7μH的电感
    * 这是目前中景园在售的黑白双色电子墨水屏, 显示效果比我在咸鱼上买的效果好很多. 虽然分辨率同为200x200,但就是效果清晰, 对比度也好, 缺点就是比我咸鱼上5块钱买的贵.
    * 由于能够正常休眠,所以不需要DCDC了, 功耗表现稍微比SSD1607版本好一丢丢

### 尺寸
* 34mm×39mm×8mm

### 耗电信息
* 休眠状态下≈3μA
* 刷新 10~20mA, 刷新完毕后会立刻进入休眠


## 时间校准说明
开机时候长按按钮, 进入时间同步模式, 并在看见如下图后松开按钮

<img src="./Image/bluetoothMode.png" width=200 title="时间同步模式的显示内容"/>

此时装置将会搜索周围的蓝牙广播.当搜索到符合约定格式的时间广播适合会自动重启.

时间广播格式为`'F' + 时间戳的16进制字符串 + 'R'`, 可以使用名为`Friday Ink时间同步`的小程序进行时间同步.

<img src="./Image/mini_app_qr_code.jpg" width=200 title="小程序二维码"/>

当进入时间同步模式后20s内无法搜索到符合要求的时间广播,会自动退出同步.


## 编译及烧录
推荐在Linux环境下进行编译, 这里我使用的是WSL2内的ubuntu子系统.
1. clone 本项目, cd进入后执行`git submodule update --init --recursive`
2. 安装Rust
3. 跟着[riscv-gnu-toolchain](https://github.com/riscv-collab/riscv-gnu-toolchain)仓库的Release界面下载riscv32-elf-ubuntu-22.04-gcc-nightly,配置好环境变量
4. 根据你的MRS_Community配置u8g2_rs内的build.rs中头文件目录
5. 根据屏幕类型,修改main.rs中创建屏幕的参数
6. 执行`cargo build-hex`获得编译好的hex文件
7. 使用WCHISPStudio工具串口模式下载得到的hex文件


## 设定集
<img src="./Image/PreviewInFusion.png" width=640/>
<img src="./Image/BackViewInFusion.png" width=640/>
<img src="./Image/PCBATop.png" width=640/>
<img src="./Image/PCBABottom.png" width=640/>

## Q&A

### Q: 时间校准时候卡在校准页怎么办
### A: 推荐提前发送时间广播, 让手机和装置足够接近. 然后装置进入校准, 并在发现20S后未能自动退出校准或按钮无法强制退出校准时, 取下电池, 重新安装电池.

## 已知问题
* 时间校准有一定概率卡死在校准页
    * 考虑引入看门狗, 看门狗超时复位
* FPC座HC-FPC-05-09-24RLTAG和我自己在淘宝买的不一样, 但是接触更加牢靠.代价是墨水屏排线超出PCB范围了

## 参考资料

* [ch58x-hal](https://github.com/ch32-rs/ch58x-hal)
* CH582F原理图参考azunya的[纽扣电池蓝牙温湿度计CR1220+CH592F+SHT40](https://oshwhub.com/azunya/ch592f-cr1220)
* SSD1681的驱动来自pikot的[U8g2_Arduino](https://github.com/pikot/U8g2_Arduino.git)