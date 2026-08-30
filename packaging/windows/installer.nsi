Unicode True

!ifndef VERSION
  !error "Falta VERSION"
!endif
!ifndef BINARY
  !error "Falta BINARY"
!endif
!ifndef LICENSE
  !error "Falta LICENSE"
!endif
!ifndef OUTPUT
  !error "Falta OUTPUT"
!endif

Name "Lienzo ${VERSION}"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\Lienzo"
RequestExecutionLevel user

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "Lienzo"
  SetOutPath "$INSTDIR"
  File /oname=Lienzo.exe "${BINARY}"
  File /oname=LICENSE.txt "${LICENSE}"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Lienzo"
  CreateShortcut "$SMPROGRAMS\Lienzo\Lienzo.lnk" "$INSTDIR\Lienzo.exe"
  CreateShortcut "$SMPROGRAMS\Lienzo\Desinstalar Lienzo.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\Lienzo.lnk" "$INSTDIR\Lienzo.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "DisplayName" "Lienzo ${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "Publisher" "poncho-ajmv"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "UninstallString" '"$INSTDIR\Uninstall.exe"'
SectionEnd

Section "Uninstall"
  Delete "$DESKTOP\Lienzo.lnk"
  Delete "$SMPROGRAMS\Lienzo\Lienzo.lnk"
  Delete "$SMPROGRAMS\Lienzo\Desinstalar Lienzo.lnk"
  RMDir "$SMPROGRAMS\Lienzo"
  Delete "$INSTDIR\Lienzo.exe"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo"
SectionEnd
