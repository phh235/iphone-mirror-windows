"""Generate deterministic WiX v3 input for a per-user x64 installer."""
import argparse,hashlib
from pathlib import Path
import xml.etree.ElementTree as ET
ROOT=Path(__file__).resolve().parents[1]
NS="http://schemas.microsoft.com/wix/2006/wi"
ET.register_namespace("",NS)
def add(parent,tag,**attrs):
    if tag=="Component" and attrs.get("Guid")=="*":
        import uuid
        attrs["Guid"]=str(uuid.uuid5(uuid.UUID("395fe87f-28d3-4a7b-a13c-d3137c2ca0ba"),"iMirror:x64:"+attrs["Id"])).upper()
    return ET.SubElement(parent,"{"+NS+"}"+tag,attrs)
def key(prefix,value):return prefix+hashlib.sha256(value.encode()).hexdigest()[:24]
def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("--stage",type=Path,required=True)
    ap.add_argument("--output",type=Path,required=True)
    ap.add_argument("--version",default="0.1.0")
    a=ap.parse_args()
    root=ET.Element("{"+NS+"}Wix")
    product=add(root,"Product",Id="*",Name="iMirror",Language="1033",Version=a.version,Manufacturer="iMirror Project",
        UpgradeCode="C608108E-14F9-42DB-9E7D-A82194E6CBBD")
    add(product,"Package",InstallerVersion="500",Compressed="yes",InstallScope="perUser",InstallPrivileges="limited",Platform="x64",
        Description="Native iPhone mirroring for Windows",Comments="Engineering preview; see VALIDATION.md for outstanding release gates.")
    add(product,"MajorUpgrade",DowngradeErrorMessage="A newer version of iMirror is already installed.")
    add(product,"MediaTemplate",EmbedCab="yes",CompressionLevel="high")
    build_property=add(product,"Property",Id="IMIRROR_WINDOWS_BUILD")
    add(build_property,"RegistrySearch",Id="WindowsBuildSearch",Root="HKLM",Key=r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",Name="CurrentBuildNumber",Type="raw",Win64="yes")
    add(product,"Condition",Message="iMirror requires Windows 11 x64 or Windows 10 22H2 x64.").text="Installed OR (VersionNT64 AND IMIRROR_WINDOWS_BUILD >= 19045)"
    add(product,"Property",Id="ARPNOMODIFY",Value="1")
    add(product,"Property",Id="WIXUI_INSTALLDIR",Value="INSTALLFOLDER")
    add(product,"WixVariable",Id="WixUILicenseRtf",Value=str(ROOT/"installer/license.rtf"))
    add(product,"UIRef",Id="WixUI_FeatureTree")
    tree=add(product,"Directory",Id="TARGETDIR",Name="SourceDir")
    local=add(tree,"Directory",Id="LocalAppDataFolder")
    programs=add(local,"Directory",Id="UserPrograms",Name="Programs")
    install=add(programs,"Directory",Id="INSTALLFOLDER",Name="iMirror")
    feature=add(product,"Feature",Id="MainFeature",Title="iMirror",Level="1",Absent="disallow",Display="expand")
    directories={"":install}
    removed=set()
    for source in sorted(a.stage.rglob("*")):
        if not source.is_file():continue
        rel=source.relative_to(a.stage)
        parent=""
        for part in rel.parts[:-1]:
            child=(parent+"/" if parent else "")+part
            if child not in directories:
                directories[child]=add(directories[parent],"Directory",Id=key("D",child),Name=part)
            parent=child
        cid=key("C",rel.as_posix())
        component=add(directories[parent],"Component",Id=cid,Guid="*",Win64="yes")
        add(component,"RegistryValue",Root="HKCU",Key="Software\\iMirror\\Installer",Name=cid,Type="integer",Value="1",KeyPath="yes")
        fid="AppExe" if rel.as_posix()=="iMirror.exe" else key("F",rel.as_posix())
        add(component,"File",Id=fid,Source=str(source),Name=source.name)
        if parent not in removed:
            add(component,"RemoveFolder",Id=key("R",parent),On="uninstall")
            removed.add(parent)
        add(feature,"ComponentRef",Id=cid)
    menu=add(tree,"Directory",Id="ProgramMenuFolder")
    shortcut_dir=add(menu,"Directory",Id="ApplicationProgramsFolder",Name="iMirror")
    component=add(shortcut_dir,"Component",Id="StartMenuShortcuts",Guid="*",Win64="yes")
    add(component,"Shortcut",Id="StartMenuApp",Name="iMirror",Target="[INSTALLFOLDER]iMirror.exe",WorkingDirectory="INSTALLFOLDER")
    add(component,"RemoveFolder",Id="RemoveStartMenuFolder",On="uninstall")
    add(component,"RegistryValue",Root="HKCU",Key="Software\\iMirror\\Installer",Name="StartMenu",Type="integer",Value="1",KeyPath="yes")
    for directory, element in directories.items():
        if directory not in removed:
            add(component,"RemoveFolder",Id=key("R",directory),Directory=element.attrib["Id"],On="uninstall")
    add(component,"RemoveFolder",Id="RemoveUserPrograms",Directory="UserPrograms",On="uninstall")
    add(feature,"ComponentRef",Id="StartMenuShortcuts")
    desktop=add(tree,"Directory",Id="DesktopFolder")
    component=add(desktop,"Component",Id="DesktopShortcut",Guid="*",Win64="yes")
    add(component,"Shortcut",Id="DesktopApp",Name="iMirror",Target="[INSTALLFOLDER]iMirror.exe",WorkingDirectory="INSTALLFOLDER")
    add(component,"RegistryValue",Root="HKCU",Key="Software\\iMirror\\Installer",Name="Desktop",Type="integer",Value="1",KeyPath="yes")
    option=add(product,"Feature",Id="DesktopFeature",Title="Desktop shortcut",Description="Add an iMirror shortcut to the desktop.",Level="2")
    add(option,"ComponentRef",Id="DesktopShortcut")
    ET.indent(root)
    ET.ElementTree(root).write(a.output,encoding="utf-8",xml_declaration=True)
if __name__=="__main__":main()
