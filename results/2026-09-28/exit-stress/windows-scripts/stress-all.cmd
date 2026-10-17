@echo off
rem Runs every Windows exit-stress set in order.
set "ROOT=%~dp0.."
powershell -NoProfile -Command "(Get-Date).ToString(\"o\")" > "%ROOT%\stress\start-time.txt"
call "%~dp0stress-set.cmd" followup exit-immediately 200 windows-followup-exit-immediately
call "%~dp0stress-set.cmd" candidate exit-immediately 50 windows-control-adde532c9-exit-immediately
call "%~dp0stress-set.cmd" followup exit-after-backend 200 windows-followup-exit-after-backend
