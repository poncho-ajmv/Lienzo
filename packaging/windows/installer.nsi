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
!ifndef ICON
  !error "Falta ICON"
!endif

!include "MUI2.nsh"
!include "WordFunc.nsh"

!define MUI_ABORTWARNING
!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_FINISHPAGE_RUN "$INSTDIR\Lienzo.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Abrir Lienzo"

Name "Lienzo ${VERSION}"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\Lienzo"
InstallDirRegKey HKCU "Software\poncho-ajmv\Lienzo" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
ManifestDPIAware true
BrandingText "Lienzo ${VERSION}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "Lienzo"
VIAddVersionKey "FileDescription" "Instalador de Lienzo"
VIAddVersionKey "CompanyName" "poncho-ajmv"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "OriginalFilename" "Lienzo-Setup.exe"
VIAddVersionKey "LegalCopyright" "Copyright © 2026 poncho-ajmv"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${LICENSE}"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "Spanish"

Function .onInit
  IfSilent continuar
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "DisplayVersion"
  StrCmp $0 "" continuar

  ${VersionCompare} "$0" "${VERSION}" $1
  StrCmp $1 "2" actualizar
  StrCmp $1 "0" reinstalar

  MessageBox MB_YESNO|MB_ICONEXCLAMATION \
    "Ya está instalado Lienzo $0, que es más reciente que ${VERSION}.$\r$\n$\r$\n¿Desea instalar esta versión de todos modos?" \
    IDYES continuar
  Abort

actualizar:
  MessageBox MB_OKCANCEL|MB_ICONINFORMATION \
    "Se encontró Lienzo $0.$\r$\n$\r$\nCierre Lienzo si está abierto. El instalador lo actualizará a Lienzo ${VERSION} conservando sus preferencias." \
    IDOK continuar
  Abort

reinstalar:
  MessageBox MB_OKCANCEL|MB_ICONINFORMATION \
    "Lienzo ${VERSION} ya está instalado.$\r$\n$\r$\nCierre Lienzo si está abierto. El instalador reparará la instalación existente." \
    IDOK continuar
  Abort

continuar:
FunctionEnd

Section "Lienzo" SEC_LIENZO
  SetShellVarContext current
  SetOverwrite on
  SetOutPath "$INSTDIR"
  File /oname=Lienzo.exe "${BINARY}"
  File /oname=LICENSE.txt "${LICENSE}"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateDirectory "$SMPROGRAMS\Lienzo"
  CreateShortcut "$SMPROGRAMS\Lienzo\Lienzo.lnk" "$INSTDIR\Lienzo.exe"
  CreateShortcut "$SMPROGRAMS\Lienzo\Desinstalar Lienzo.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\Lienzo.lnk" "$INSTDIR\Lienzo.exe"

  WriteRegStr HKCU "Software\poncho-ajmv\Lienzo" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "DisplayName" "Lienzo ${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "DisplayIcon" "$INSTDIR\Lienzo.exe,0"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "Publisher" "poncho-ajmv"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "URLInfoAbout" "https://github.com/poncho-ajmv/Lienzo"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  Delete "$DESKTOP\Lienzo.lnk"
  Delete "$SMPROGRAMS\Lienzo\Lienzo.lnk"
  Delete "$SMPROGRAMS\Lienzo\Desinstalar Lienzo.lnk"
  RMDir "$SMPROGRAMS\Lienzo"
  Delete "$INSTDIR\Lienzo.exe"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lienzo"
  DeleteRegKey HKCU "Software\poncho-ajmv\Lienzo"
SectionEnd
