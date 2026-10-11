# 本地聯調：拿剛編出來的 Server 與 TSF DLL，配一份裝著真實資料的安裝目錄跑起來。
#
# 為什麼不直接在倉庫裡跑：`bundled_root()` 按 exe 位置找資料，`target\debug\` 下會落到倉庫根，
# 而 `data\generated\`（詞庫 / LM / 釋義表）是 gitignore 的、本機多半沒有，於是退回 `assets\sample\`
# 樣例。形碼候選照樣出得來（碼錶是獨立的），但**譯文幾乎全空**，會讓人誤以為釋義那條設計沒生效。
# 拷一份安裝目錄出來跑，`bundled_root()` 就落在它上，資料是真的。
#
# 為什麼要換 DLL：`qingjian_core::Candidate` 是線上格式的一部分（見 `protocol/mod.rs`），
# Core 那邊加一個 `CandidateKind` 變體，老 DLL 就解不出整條幀、把按鍵原樣放行——表現是
# 「輸入法突然只出英文」。所以兩邊必須同一份原始碼編出來的。
#
# 用法：
#   pwsh -File apps\windows\scripts\test-local.ps1
#   pwsh -File apps\windows\scripts\test-local.ps1 -SkipBuild -SkipRegister
#
# 管理員不是必須的：註冊 DLL 那一步指令碼會自己判斷，不是管理員就把命令打出來讓你自己跑。

[CmdletBinding()]
param(
    # 已安裝的目錄（真實資料從這裡拷）。
    [string]$InstallDir = "$env:ProgramFiles\Qingjian",

    # 聯呼叫的工作目錄。**不要用 %TEMP%**：它在這臺機器上是 8.3 短名（`C:\Users\TONYWU~1\...`），
    # PowerShell 走不通，`cd` 會報「An object at the specified path ... does not exist」。
    [string]$WorkDir = "$env:USERPROFILE\qingjian-devtest",

    # 跳過編譯（已經編好了）。
    [switch]$SkipBuild,

    # 跳過註冊 DLL（只想換 Server）。
    [switch]$SkipRegister
)

$ErrorActionPreference = 'Stop'

# apps\windows\scripts -> apps\windows -> apps -> 倉庫根
$repo = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
$table = Join-Path $repo 'assets\wubi\wubi86.tsv'
$server = Join-Path $repo 'target\debug\qingjian-server.exe'
$dll = Join-Path $repo 'target\debug\qingjian_tsf.dll'
$config = Join-Path $env:APPDATA 'Qingjian\config.toml'
$logs = Join-Path $env:APPDATA 'Qingjian\logs'

# 文本服務的 CLSID，與 `apps/windows/tsf/src/com/mod.rs` 的 `CLSID_QINGJIAN` 同步。
# 註冊完查這裡指向哪兒——比 regsvr32 的退出碼可靠（見下）。
$TsfClsid = '{4FDCA82D-E923-49BF-9E75-BB906B93B8BB}'

function Assert-Path($path, $what) {
    if (-not (Test-Path -LiteralPath $path)) {
        throw "$what 不在：$path"
    }
}

Write-Host '== 前置检查' -ForegroundColor Cyan
Assert-Path $InstallDir '安装目录（先装一次青简，或者用 -InstallDir 指过去）'
Assert-Path $table '五笔码表（跑 dict-convert wubi 生成，见 assets/wubi/README.md）'
if (-not $SkipBuild) {
    Write-Host '   安装目录 ✓  码表 ✓'
}
else {
    Assert-Path $server 'Server 产物（去掉 -SkipBuild 先编一次）'
    Assert-Path $dll 'TSF DLL 产物（去掉 -SkipBuild 先编一次）'
}

if (-not $SkipBuild) {
    # 舊的那次註冊可能還指著 `target\debug`，TSF 會按需把 DLL 載進各個宿主（記事本、終端、資源管理器…）。
    # 被載著時編譯寫不進去，cargo 報的是「failed to remove file ... 拒絕訪問」，看著像程式碼壞了。
    # 先查出來點名，別讓人去猜。
    $holders = @(
        tasklist /m qingjian_tsf.dll /NH 2>$null |
            Select-String -SimpleMatch 'qingjian_tsf.dll'
    )
    if ($holders.Count -gt 0) {
        Write-Host '== 有进程正加载着 TSF DLL，编译写不进去' -ForegroundColor Yellow
        $holders | ForEach-Object { Write-Host "   $($_.Line.Trim())" -ForegroundColor Yellow }
        Write-Host ''
        Write-Host '   切走输入法就能放开（TSF 在切走时卸载 DLL）：Win+Space 换到别的输入法，或 Shift 切英文。' -ForegroundColor Yellow
        Write-Host '   还不行就关掉那个窗口重开，在新窗口里别切到青简，直接跑本脚本。' -ForegroundColor Yellow
        Write-Host '   （查：tasklist /m qingjian_tsf.dll）' -ForegroundColor Yellow
        exit 1
    }

    Write-Host '== 编译 Server 与 TSF DLL' -ForegroundColor Cyan
    # uiAccess manifest 預設是開的，沒簽名的 exe 帶它會直接起不來（Permission denied）；
    # 與 ci.yml 的 windows job 同一個開關
    $env:QINGJIAN_UIACCESS = '0'
    Push-Location $repo
    try {
        cargo build -p qingjian-windows-server -p qingjian-windows-tsf
        if ($LASTEXITCODE -ne 0) { throw 'cargo build 失败' }
    }
    finally {
        Pop-Location
    }
    Assert-Path $server 'Server 产物'
    Assert-Path $dll 'TSF DLL 产物'
}

Write-Host '== 停掉在跑的 Server' -ForegroundColor Cyan
# 管道名只有一個，舊的佔著不放，新起來的連不上——那是靜默的二選一，會讓人以為新程式碼沒生效
$running = Get-Process -Name qingjian-server -ErrorAction SilentlyContinue
if ($running) {
    $running | Stop-Process -Force
    Write-Host "   停掉 $($running.Count) 个"
}
else {
    Write-Host '   没有在跑的'
}

Write-Host "== 拷一份安装目录到 $WorkDir" -ForegroundColor Cyan
# 不動 Program Files：不需要管理員，也不會把「裝好的那份」搞成半新半舊
robocopy $InstallDir $WorkDir /MIR /NFL /NDL /NJH /NJS | Out-Null
if ($LASTEXITCODE -ge 8) { throw "robocopy 失败（exit $LASTEXITCODE）" }

Copy-Item -LiteralPath $server -Destination $WorkDir -Force
New-Item -ItemType Directory -Force -Path (Join-Path $WorkDir 'assets\wubi') | Out-Null
Copy-Item -LiteralPath $table -Destination (Join-Path $WorkDir 'assets\wubi') -Force
# DLL 也拷過來再註冊：從 `target\debug` 註冊的話，那個檔案會被載入它的程序鎖住，
# 之後 `cargo build` 寫不進去（連結報 LNK1104，看著像程式碼壞了）
$workDll = Join-Path $WorkDir 'qingjian_tsf.dll'
Copy-Item -LiteralPath $dll -Destination $workDll -Force
Write-Host '   新 Server、TSF DLL 与五笔码表已就位'

$elevated = [Security.Principal.WindowsPrincipal]::new(
    [Security.Principal.WindowsIdentity]::GetCurrent()
).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $SkipRegister) {
    Write-Host '== 注册 TSF DLL' -ForegroundColor Cyan
    if ($elevated) {
        & regsvr32.exe /s $workDll
        $exit = $LASTEXITCODE
        # regsvr32 的退出碼不可信（實測註冊成功仍返回 3），所以讀登錄檔看結果。
        # 讀 HKLM 那一份：提權時 regsvr32 寫的就是它。別讀 HKCR——那是 HKLM + HKCU 的合併檢視，
        # 實測給過假警報，把一次成功的註冊報成失敗、白停一輪。
        $registered = (
            Get-ItemProperty `
                -LiteralPath "Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$TsfClsid\InprocServer32" `
                -ErrorAction SilentlyContinue
        ).'(default)'
        if ($registered -ieq $workDll) {
            Write-Host "   已注册新 DLL（regsvr32 退出码 $exit，不代表失败）"
        }
        else {
            # 只警告不中斷：這個判據本身出過假警報，中斷的代價是白跑一整輪
            Write-Host "   ！注册可能没生效：InprocServer32 = «$registered»，期望 «$workDll»（regsvr32 退出码 $exit）" -ForegroundColor Yellow
            Write-Host '     先继续；输入法不出候选时再手工 regsvr32 一次。' -ForegroundColor Yellow
        }
    }
    else {
        Write-Host '   不是管理员，请另开一个管理员 PowerShell 跑：' -ForegroundColor Yellow
        Write-Host "     regsvr32 `"$workDll`"" -ForegroundColor Yellow
    }
}

Write-Host '== 起 Server' -ForegroundColor Cyan
Start-Process -FilePath (Join-Path $WorkDir 'qingjian-server.exe') -WorkingDirectory $WorkDir
Start-Sleep -Seconds 3

Write-Host ''
Write-Host '接下来手动做这四步：' -ForegroundColor Cyan
Write-Host ''
Write-Host '  1. 看日志确认两边都对了（应该有两行「形码码表已载入」与「协议」相关的警告不该出现）：'
Write-Host "       Get-Content `"$logs\qingjian-server.*.log`" -Tail 20"
Write-Host '     entries=89256 是码表；词库那行应该是一万以上，不是 148（148 说明落到样例数据了）。'
Write-Host ''
Write-Host '  2. 改配置（保存即热加载，不用重启）。拼音与形码是两条独立的轴：'
Write-Host "       $config"
Write-Host '       只用五笔           scheme = "none"    wubi = "wubi86"'
Write-Host '       五笔 + 全拼混输     scheme = "pinyin"  wubi = "wubi86"'
Write-Host '       只用双拼           scheme = "xiaohe"  wubi = ""'
Write-Host ''
Write-Host '  3. 关掉记事本再打开（已经在跑的进程手里攥着旧 DLL，注册新的对它没用），切到青简。'
Write-Host ''
Write-Host '  4. 敲这几组：'
Write-Host '       r      → 的（一级简码）'
Write-Host '       v      → 发（重点：不该冒出「v1+2 → 3」那种算式候选）'
Write-Host '       ga     → 开 排在 开发 前面'
Write-Host '       khlg   → 中国'
Write-Host '       ggll   → 一'
Write-Host '       混输时：nihao 出「你好」（五笔查不到就落到拼音），第 5 个字母起自动只剩拼音'
Write-Host '     再看候选右侧有没有译文小字、悬浮状态条第一格是不是方案名（混输是「五笔（86） + 全拼」）。'
Write-Host ''
$installedDll = Get-ChildItem -LiteralPath $InstallDir -Filter 'qingjian_tsf-*.dll' -ErrorAction SilentlyContinue |
    Select-Object -First 1 -ExpandProperty FullName
Write-Host '回滚：注销新 DLL，再注册装好的那个'
Write-Host "      regsvr32 /u `"$workDll`""
if ($installedDll) {
    Write-Host "      regsvr32 `"$installedDll`""
}
else {
    Write-Host "      （$InstallDir 下没找到 qingjian_tsf-*.dll，回滚时用安装包重装一遍）"
}

#regsvr32 "C:\OpenSource\qingjian\target\debug\qingjian_tsf.dll"