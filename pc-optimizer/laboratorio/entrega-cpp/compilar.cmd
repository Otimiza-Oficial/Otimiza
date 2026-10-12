@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d "%~dp0"
cl /nologo /EHsc /std:c++20 /O2 /MT /utf-8 teste_presentation.cpp /Fe:teste_presentation.exe
cl /nologo /EHsc /std:c++20 /O2 /MT /utf-8 apresentador.cpp /Fe:apresentador.exe
