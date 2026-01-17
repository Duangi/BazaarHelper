!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Copying OpenCV DLL..."
  SetOutPath $INSTDIR
  File "/oname=opencv_world4120.dll" "C:\opencv\build\x64\vc16\bin\opencv_world4120.dll"
!macroend