@echo off
setlocal
cd /d "%~dp0.."
py -m PyInstaller --noconfirm --clean --onefile --console --name HuntLog-Website --paths tools --add-data "history;history" --distpath build --workpath build\website-work --specpath build tools\website.py
