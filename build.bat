@echo off
set DEST_DIR=c:\TOOLS\exe

for /f "tokens=* delims=" %%a in ('date /t') do set current_date=%%a
for /f "tokens=* delims=" %%b in ('time /t') do set current_time=%%b

:: Make sure the destination folder exists, if not, create it first
if not exist "%DEST_DIR%" mkdir "%DEST_DIR%"

echo Building...
cargo build --release

if not %errorlevel%==0 (
    echo.
    echo [ERROR] ❌ BUILD FAILURE! Please improve your code.
    echo.
    sendgrowl.exe "Rust-Builder" build "FAILED %current_date% %current_time%" "[%current_date% %current_time%] ❌ Build successful: rund.exe" -i "c:\TOOLS\exe\rust.png" -H 127.0.0.1
    :: pause
    exit /b %errorlevel%
) else (
    echo.
    echo Build Success! Copying files...
    copy /y target\release\rund.exe "%DEST_DIR%"
    
    copy /y target\release\rund.exe .
    
    mkdir test
    
    copy /y target\release\rund.exe test
    
    cd test
    ls
    
    echo.
    echo [COMPLETE] ✅ Files successfully copied to %DEST_DIR%

    sendgrowl.exe "Rust-Builder" build "SUCCESS %current_date% %current_time%" "[%current_date% %current_time%] ✅ Build successful: rund.exe" -i "c:\TOOLS\exe\rust.png" -H 127.0.0.1
    
)
