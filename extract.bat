cd "..\Il2CppDumper\Il2CppDumper\bin\Debug\net8.0\"
"Il2CppDumper.exe" "C:\Program Files\Epic Games\BloonsTD6\GameAssembly.dll" "C:\Program Files\Epic Games\BloonsTD6\BloonsTD6_Data\il2cpp_data\Metadata\global-metadata.dat"
copy "dump.cs" "..\..\..\..\..\testing\btd6-tool-bindgen\"

rem cat script.json | jq '.ScriptMetadata[] | select(.Name == "Assets.Scripts.Unity.UI_New.InGame.InGame_TypeInfo") | .Address'
