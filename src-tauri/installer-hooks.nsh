; Remove only this user's assistant logon task. AppData is retained.
!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -Command "$$ErrorActionPreference=$\'Stop$\'; $$name=$\'IdentityV-Assistant-$\'+[System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value; $$task=Get-ScheduledTask -TaskPath $\'\$\' | Where-Object { $$_.TaskName -eq $$name }; if ($$task) { $$task | Unregister-ScheduledTask -Confirm:$$false }"'
  Pop $0
  DetailPrint "Logon task cleanup exit code: $0"
!macroend
