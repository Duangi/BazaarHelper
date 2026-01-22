!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Copying OpenCV DLL..."
  SetOutPath $INSTDIR
  File "/oname=opencv_world4120.dll" "C:\opencv\build\x64\vc16\bin\opencv_world4120.dll"
  File "/oname=DirectML.dll" "D:\Projects\BazaarHelper\src-tauri\DirectML.dll"
  File "/oname=onnxruntime.dll" "D:\Projects\BazaarHelper\src-tauri\onnxruntime.dll"
  File "/oname=onnxruntime_providers_shared.dll" "D:\Projects\BazaarHelper\src-tauri\onnxruntime_providers_shared.dll"
  
!macroend