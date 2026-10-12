; 青簡 Windows 輸入法安裝指令碼（Inno Setup）。
;
; 裝到 Program Files\Qingjian（64 位），把 TSF DLL（64 位與 32 位各一份，見 README「安裝佈局」）、Server、設定程式與隨包資料裝在一起，
; 然後：① 給安裝目錄加 ALL APPLICATION PACKAGES 讀+執行許可權（UWP/AppContainer 應用——工作列搜尋、
; 設定——才能載入 DLL）；② regsvr32 註冊文本服務，64 位與 32 位各註冊一次（圖示落到 %ProgramData%\Qingjian）；
; ③ 在「啟動」資料夾放 Server 快捷方式（登入時由 Explorer 走 ShellExecute 拉起，uiAccess 才生效——
;    計劃任務直接拉起拿不到 uiAccess）；④ 裝完點 Finish 立即以原使用者 ShellExecute 起一次 Server，免得先登出。
; 解除安裝反向：刪舊任務（若有）、殺 Server、反註冊 DLL，再刪檔案（使用者資料 %APPDATA%\Qingjian 保留；啟動快捷方式 Inno 自動刪）。
;
; 升級：DLL 被載入進每個應用程序，檔案鎖著覆蓋不了，所以 DLL 按版本起名（qingjian_tsf-<版本>.dll）並排裝，
; 註冊新的，舊的裝完後刪（刪不掉的登記成重啟後刪）；已開著的應用繼續用舊 DLL 直到重啟，Server 兩個版本都服務。
; Inno 的 CloseApplications 會用 Restart Manager 找出所有載入了 *.dll 的程序要求關閉——對輸入法 DLL 就是關一切，故關掉；
; 只有 Server / 設定程式兩個 exe 要覆蓋，安裝前自己 taskkill。
; Server 殺掉後，正在打字的應用裡 DLL 連不上會自己把它拉起來（見 tsf 的 launch.rs），又佔住 exe 和 mmap 著的資料：
; 安裝 / 解除安裝期間持有互斥體 Global\QingjianInstaller，新 DLL 看到它就不拉；舊版 DLL 不認得它，
; 所以 Server exe 先改名騰位再殺、[Files] 裡排最後裝，新 exe 落地前 DLL 拉不起任何 Server。
;
; 版本號由打包指令碼用 /DAppVersion=... 傳入，預設 0.1.0。用法見本目錄 README.md。

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
; VersionInfoVersion 只認 a.b.c.d 數字；版本帶預釋出字尾（0.1.0-alpha.1）時由打包指令碼傳去掉字尾的數字版本。
#ifndef AppVersionNumeric
  #define AppVersionNumeric AppVersion
#endif
#define AppName "青简"
#define Publisher "青简"
#define WebsiteUrl "https://qingjian.im"
; 指令碼相對倉庫根（ime/）：installer → windows → apps → ime
#define Repo "..\..\.."
; 按版本起名的 TSF DLL（見檔案頭「升級」）。
#define TsfDll "qingjian_tsf-" + AppVersion + ".dll"
#define TsfDll32 "qingjian_tsf-" + AppVersion + "-x86.dll"

[Setup]
AppId={{A7E3C1F2-5B94-4D6A-9C0E-2F8B1D3A6E70}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#Publisher}
AppSupportURL={#WebsiteUrl}
VersionInfoVersion={#AppVersionNumeric}
DefaultDirName={autopf}\Qingjian
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Windows 10 1809 起（Windows App Runtime 的下限，見 docs\notes\windows-win10.md）。
MinVersion=10.0.17763
PrivilegesRequired=admin
; 別讓 Restart Manager 去關所有載入了 DLL 的應用（那是每一個有文本框的應用）。
CloseApplications=no
OutputDir={#Repo}\target\installer
OutputBaseFilename=qingjian-{#AppVersion}-windows-x86_64-setup
SetupIconFile={#Repo}\apps\windows\tsf\resources\qingjian.ico
UninstallDisplayIcon={app}\qingjian.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; 按系統介面語言挑安裝語言（簡體 / 繁體），對不上才問。
ShowLanguageDialog=auto

[Languages]
; 簡體與繁體兩套介面文案（Inno Setup 7.0.2 起兩份翻譯都隨編譯器帶，見 build.ps1 對 ISCC 版本的要求）。
; 快捷方式、程式組、啟動項的名字不隨語言變：升級時換了語言會留下另一套同功能的快捷方式（啟動項會起兩個 Server）。
Name: "chs"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"
Name: "cht"; MessagesFile: "compiler:Languages\ChineseTraditional.isl"

[Messages]
; 完成頁：DLL 會裝進每個應用程序，裝之前就開著的應用要用新版必須重啟（見檔案頭「升級」）。
; 這是最容易被誤解成「設定/新版沒生效」的一點，所以放在完成頁明說。
chs.FinishedLabel=安装完成。请注销后重新登录（或重启电脑），青简才会在所有应用里生效。%n%n不方便注销的话，先关掉再重新打开要打字的应用也可以。%n%n本版本只支持繁体中文：青简只登记在「中文(台灣)」语言下，有「青簡」（拼音）与「青簡注音」（注音）两个，都输出繁体。系统没装「中文(台灣)」语言的，请先到「设置 → 时间和语言 → 语言和区域 → 添加语言」加上，再在它的「语言选项 → 添加键盘」里加入青简。
cht.FinishedLabel=安裝完成。請登出後重新登入（或重新啟動電腦），青簡才會在所有應用程式裡生效。%n%n不方便登出的話，先關掉再重新開啟要打字的應用程式也可以。%n%n本版本只支援繁體中文：青簡只登記在「中文(台灣)」語言下，有「青簡」（拼音）與「青簡注音」（注音，鍵盤排列可在設定裡選標準或倚天）兩個，都輸出繁體字。若輸入法列表裡沒有自動出現，從「設定 → 時間與語言 → 語言與地區 → 中文(台灣) → 語言選項 → 新增鍵盤」加入。

[CustomMessages]
chs.StatusAppContainer=配置应用容器权限…
cht.StatusAppContainer=設定應用程式容器權限…
chs.StatusRegister=注册输入法…
cht.StatusRegister=註冊輸入法…
chs.StatusRegister32=注册输入法（32 位）…
cht.StatusRegister32=註冊輸入法（32 位元）…

[Files]
; —— 二進位制 ——
; DLL 按版本起名並排裝；解除安裝時若仍被佔用，登記成重啟後刪。
Source: "{#Repo}\target\release\qingjian_tsf.dll";      DestDir: "{app}"; DestName: "{#TsfDll}"; Flags: ignoreversion uninsrestartdelete
Source: "{#Repo}\target\i686-pc-windows-msvc\release\qingjian_tsf.dll"; DestDir: "{app}"; DestName: "{#TsfDll32}"; Flags: ignoreversion uninsrestartdelete
Source: "{#Repo}\target\release\qingjian-settings.exe"; DestDir: "{app}"; Flags: ignoreversion
; 設定程式自帶一份 Windows App Runtime（自包含部署：Windows 10 上機器裝的框架包用不了，見 docs\notes\windows-win10.md）；
; 檔案由 build.ps1 按 settings-runtime.txt 從 target\release 挑進 target\installer\settings-runtime，必須與 exe 同級。
Source: "{#Repo}\target\installer\settings-runtime\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#Repo}\apps\windows\tsf\resources\qingjian.ico"; DestDir: "{app}"; Flags: ignoreversion
; —— 隨包生成資料（只裝執行時要的 .qj / .tsv，不裝 dev 中間產物）——
Source: "{#Repo}\data\generated\dict.qj";        DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\lm.qj";          DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\glossary-en.qj"; DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\glossary-ja.qj"; DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\glossary-zh.qj"; DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\glossary-es.qj"; DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\glossary-vi.qj"; DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\english.tsv";    DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\dicts\*.qj";     DestDir: "{app}\data\generated\dicts";  Flags: ignoreversion
; —— 隨包輔碼碼錶（筆畫，開箱可用）：Server 掃 data\generated\codes\；CNS11643 筆順資料派生，署名見「關於」頁 ——
;    缺表時不阻塞打包（skipifsourcedoesntexist），但釋出前應先跑 dict-convert pack codes 生成它
Source: "{#Repo}\data\generated\codes\*.qj";   DestDir: "{app}\data\generated\codes"; Flags: ignoreversion skipifsourcedoesntexist
Source: "{#Repo}\assets\stroke\LICENSE-CNS11643.txt"; DestDir: "{app}\data\generated\codes"; Flags: ignoreversion
; —— 含章·知微與含章·通變模型 ——
Source: "{#Repo}\data\models\hanzhang-zhiwei\hanzhang-zhiwei-small.qjm"; DestDir: "{app}\data\models\hanzhang-zhiwei"; Flags: ignoreversion skipifsourcedoesntexist
Source: "{#Repo}\data\models\hanzhang-tongbian\hanzhang-tongbian-small.qjm"; DestDir: "{app}\data\models\hanzhang-tongbian"; Flags: ignoreversion skipifsourcedoesntexist
; —— 隨 git 的資源 ——
Source: "{#Repo}\assets\emoji\emoji-zh.tsv";     DestDir: "{app}\assets\emoji";  Flags: ignoreversion
Source: "{#Repo}\assets\emoji\emoji-en.tsv";     DestDir: "{app}\assets\emoji";  Flags: ignoreversion
Source: "{#Repo}\assets\levels\levels-en.tsv";   DestDir: "{app}\assets\levels"; Flags: ignoreversion
Source: "{#Repo}\assets\levels\levels-ja.tsv";   DestDir: "{app}\assets\levels"; Flags: ignoreversion
; 五筆碼錶（輸入方案選五筆時用，見 assets/wubi/README.md；極點 86 碼錶，Apache-2.0）。
; 走 assets\ 與 emoji / levels 一致，Server 的 `dispatch::code::find_code_table` 照同一個相對路徑找，
; 開發佈局（cargo run）也對得上
Source: "{#Repo}\assets\wubi\wubi86.tsv";        DestDir: "{app}\assets\wubi";   Flags: ignoreversion
Source: "{#Repo}\assets\sample\dict.tsv";        DestDir: "{app}\assets\sample"; Flags: ignoreversion
; —— Server 放最後：它一落地，舊版 DLL 就能把它拉起來並佔住資料檔案（見檔案頭）——
Source: "{#Repo}\target\release\qingjian-server.exe";   DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\青简设置"; Filename: "{app}\qingjian-settings.exe"; IconFilename: "{app}\qingjian.ico"
Name: "{group}\卸载青简"; Filename: "{uninstallexe}"
; 登入自啟：登入時 Explorer 走 ShellExecute 拉起本快捷方式 → AppInfo 授予 uiAccess，候選窗才能蓋過商店 / 工作列搜尋。
; 用 {commonstartup}（所有使用者「啟動」資料夾）而非 {userstartup}：本安裝器是 admin 機器級安裝，
; admin 模式下寫每使用者區會落到「誰提權就寫誰」的 profile（Inno 會告警且可能不是目標使用者）；
; 機器級「啟動」項對每個登入使用者都在其會話裡由該使用者的 Explorer 拉起，仍是 per-user 執行、仍授予 uiAccess。
; （計劃任務直接拉起拿不到 uiAccess，故不用 schtasks。）
Name: "{commonstartup}\青简 Server"; Filename: "{app}\qingjian-server.exe"; WorkingDir: "{app}"; IconFilename: "{app}\qingjian.ico"

[Run]
; ① UWP/AppContainer 應用要能讀安裝目錄才能載入 DLL（*S-1-15-2-1 = ALL APPLICATION PACKAGES，按 SID 與語言無關）。
Filename: "{sys}\icacls.exe"; Parameters: """{app}"" /grant *S-1-15-2-1:(OI)(CI)RX /T /C /Q"; \
  Flags: runhidden waituntilterminated; StatusMsg: "{cm:StatusAppContainer}"
; ② 註冊文本服務（寫 HKCR + 圖示到 %ProgramData%\Qingjian\qingjian.ico；語言 profile 只在「中文(台灣)」下兩個（拼音 / 注音），
;    由 DLL 的 DllRegisterServer 登記，並順手拿掉早期版本登記在「中文(簡體)」下的那個）。註冊的是本版本的 DLL，
;    InprocServer32 指向新檔案；舊版本的 DLL **不能** regsvr32 /u（那會把整個 CLSID 登出掉）。
Filename: "{sys}\regsvr32.exe"; Parameters: "/s ""{app}\{#TsfDll}"""; \
  Flags: runhidden waituntilterminated; StatusMsg: "{cm:StatusRegister}"
; 32 位那份用 SysWOW64 裡的 32 位 regsvr32 註冊，InprocServer32 才落到 WOW6432Node 下給 32 位程序用。
Filename: "{syswow64}\regsvr32.exe"; Parameters: "/s ""{app}\{#TsfDll32}"""; \
  Flags: runhidden waituntilterminated; StatusMsg: "{cm:StatusRegister32}"
; ④ 裝完立即起一次 Server 見 [Code] 的 NextButtonClick：uiAccess=true 的 exe 不能用
;    CreateProcess / runasoriginaluser 拉起（報 740），必須走 ShellExecute（等同雙擊）。

[UninstallRun]
; 反向：先刪登入任務、殺 Server / 設定程式、反註冊 DLL，Inno 再刪檔案（DLL 若仍被佔用，重啟後刪）。
Filename: "{sys}\schtasks.exe"; Parameters: "/delete /tn ""Qingjian Server"" /f"; \
  Flags: runhidden; RunOnceId: "DelLogonTask"
Filename: "{sys}\taskkill.exe"; Parameters: "/im qingjian-server.exe /f"; \
  Flags: runhidden; RunOnceId: "KillServer"
Filename: "{sys}\taskkill.exe"; Parameters: "/im qingjian-settings.exe /f"; \
  Flags: runhidden; RunOnceId: "KillSettings"
Filename: "{sys}\regsvr32.exe"; Parameters: "/u /s ""{app}\{#TsfDll}"""; \
  Flags: runhidden; RunOnceId: "UnregDll"
Filename: "{syswow64}\regsvr32.exe"; Parameters: "/u /s ""{app}\{#TsfDll32}"""; \
  Flags: runhidden; RunOnceId: "UnregDll32"

[InstallDelete]
; 更早版本裝在當前使用者「啟動」資料夾裡的自啟快捷方式：與機器級那份並存會起兩個 Server（兩條狀態條）。
Type: files; Name: "{userstartup}\Qingjian Server.lnk"
; 更早版本裝的模型三件套（現在只帶正式命名的 .qjm）：留著白佔 56 MB。
Type: files; Name: "{app}\data\model\model.safetensors"
Type: files; Name: "{app}\data\model\config.json"
Type: files; Name: "{app}\data\model\vocab.json"
; 0.1.4 開發版的隨包碼錶舊位置
Type: filesandordirs; Name: "{app}\codes"

[UninstallDelete]
; 歷次升級留下的舊版本 DLL（正常在升級時就刪了；仍被佔用的會留到這裡）。
Type: files; Name: "{app}\qingjian_tsf-*.dll"
Type: files; Name: "{app}\qingjian-server.old-*.exe"

[Code]
function CreateMutex(Attributes: Longint; InitialOwner: BOOL; Name: String): THandle;
  external 'CreateMutexW@kernel32.dll stdcall';

{ 安装 / 卸载期间持有的互斥体，名字与 tsf 的 launch.rs 一致；句柄不关，进程退出时系统收回。 }
procedure HoldInstallerMutex;
begin
  if CreateMutex(0, False, 'Global\QingjianInstaller') = 0 then
    Log('建安装互斥体失败');
end;

function InitializeSetup: Boolean;
begin
  HoldInstallerMutex;
  Result := True;
end;

function InitializeUninstall: Boolean;
begin
  HoldInstallerMutex;
  Result := True;
end;

{ 运行中的 exe 不能覆盖但能改名：改成 qingjian-server.old-<随机>.exe 腾出名字，旧版 DLL 就拉不起它（见文件头）。
  装完由 DeleteStaleFiles 删掉。 }
procedure RetireServerExe;
var
  Path: String;
begin
  Path := ExpandConstant('{app}\qingjian-server.exe');
  if FileExists(Path) then
    if not RenameFile(Path, ExpandConstant('{app}\qingjian-server.old-') + IntToStr(Random(1000000)) + '.exe') then
      Log('改名旧 Server 失败: ' + Path);
end;

procedure KillProcess(const Image: String);
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/im ' + Image + ' /f', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

{ 同版本重装（开发期反复装）：目标文件名与已加载的 DLL 撞名，覆盖不了但 Windows 允许改名，
  先把它改成 qingjian_tsf-<版本>.old-<随机>.dll 腾出名字，装完由 DeleteStaleDlls 删掉 / 登记重启后删。 }
procedure RetireLoadedDll(const Name: String);
var
  Path, Retired: String;
begin
  Path := ExpandConstant('{app}\') + Name;
  if FileExists(Path) then
  begin
    Retired := ExpandConstant('{app}\qingjian_tsf-{#AppVersion}.old-') + IntToStr(Random(1000000)) + '.dll';
    if not RenameFile(Path, Retired) then
      Log('改名旧 DLL 失败: ' + Path);
  end;
end;

{ 覆盖前先结束 Server 与设置程序（只有这两个 exe 要覆盖；DLL 按版本并排装，不用关应用）。
  没在跑时 taskkill 返回非 0，忽略。 }
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  RetireServerExe;
  KillProcess('qingjian-server.exe');
  KillProcess('qingjian-settings.exe');
  RetireLoadedDll('{#TsfDll}');
  RetireLoadedDll('{#TsfDll32}');
  Result := '';
end;

{ 清掉旧版本建的「登录自启」计划任务（现在改用「启动」文件夹快捷方式，见 [Icons]）。
  计划任务直接拉起 Server 拿不到 uiAccess，升级安装时删掉它，免得它在登录时抢先以非 uiAccess 方式
  起 Server 并占住命名管道，让快捷方式那份起不来。没有旧任务时 schtasks 返回非 0，忽略即可。 }
procedure DeleteLegacyLogonTask;
var
  ResultCode: Integer;
begin
  Exec('schtasks.exe', '/delete /tn "Qingjian Server" /f', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

{ 删掉改名腾位的旧 Server；删不掉的登记成重启后删。 }
procedure DeleteRetiredServers;
var
  Dir, Path: String;
  Found: TFindRec;
begin
  Dir := ExpandConstant('{app}');
  if FindFirst(Dir + '\qingjian-server.old-*.exe', Found) then
  begin
    try
      repeat
        Path := Dir + '\' + Found.Name;
        if not DeleteFile(Path) then
          RestartReplace(Path, '');
      until not FindNext(Found);
    finally
      FindClose(Found);
    end;
  end;
end;

{ 删掉旧版本的 DLL（含没带版本号的最早那份）。仍被某个应用加载着的删不掉，登记成重启后删：
  那些应用重启前继续用旧 DLL，Server 两个版本都服务。 }
procedure DeleteStaleDlls;
var
  Dir, Path: String;
  Found: TFindRec;
begin
  Dir := ExpandConstant('{app}');
  if FindFirst(Dir + '\qingjian_tsf*.dll', Found) then
  begin
    try
      repeat
        if (CompareText(Found.Name, '{#TsfDll}') <> 0) and (CompareText(Found.Name, '{#TsfDll32}') <> 0) then
        begin
          Path := Dir + '\' + Found.Name;
          if not DeleteFile(Path) then
            RestartReplace(Path, '');
        end;
      until not FindNext(Found);
    finally
      FindClose(Found);
    end;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    DeleteLegacyLogonTask;
    DeleteStaleDlls;
    DeleteRetiredServers;
  end;
end;

{ 卸载同理：[UninstallRun] 杀 Server 之前先改名，旧版 DLL 拉不起它，文件才删得掉。 }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
    RetireServerExe;
end;

{ 装完在完成页点 Finish 后立即起一次 Server。
  uiAccess=true 的 exe 不能用 CreateProcess / runasoriginaluser 拉起（报 740），
  必须以原（非提升）用户身份 ShellExecute（等同双击），AppInfo 才会授予 uiAccess 高 z-band 权限。 }
function NextButtonClick(CurPageID: Integer): Boolean;
var
  ErrorCode: Integer;
begin
  Result := True;
  if (CurPageID = wpFinished) and (not WizardSilent) then
    ShellExecAsOriginalUser(
      '', ExpandConstant('{app}\qingjian-server.exe'), '', '',
      SW_SHOWNORMAL, ewNoWait, ErrorCode);
end;
