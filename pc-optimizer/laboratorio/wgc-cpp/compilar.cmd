@echo off
rem Compila o teste da Windows.Graphics.Capture em C++/WinRT com o MSVC das Build Tools 2022.
rem /MT: runtime embutido, o executavel nao pede o Visual C++ Redistributable no PC de ninguem.
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d "%~dp0"
cl /nologo /EHsc /std:c++20 /O2 /MT /utf-8 teste_wgc.cpp /Fe:teste_wgc.exe /link user32.lib
