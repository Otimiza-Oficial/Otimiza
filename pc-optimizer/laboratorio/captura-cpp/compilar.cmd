@echo off
rem Compila a bancada de captura em C++ com o MSVC das Build Tools 2022 (/MT: runtime embutido).
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d "%~dp0"
cl /nologo /EHsc /std:c++20 /O2 /MT /utf-8 bancada_captura.cpp /Fe:bancada_captura.exe /link user32.lib
