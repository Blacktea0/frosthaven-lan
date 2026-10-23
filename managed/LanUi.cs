using System;using System.Collections.Generic;using HarmonyLib;
namespace FrosthavenLan {
 public static class LanUi {
  public static bool Enabled {get;private set;}
  static readonly Dictionary<object,string> Originals=new Dictionary<object,string>();
  static Type VersionComponent {get{return Payload.Game.GetType("VersionAndChangeLog",true);}}
  public static void Install(Harmony harmony){
   foreach(string method in new[]{"Awake","HandleOnLanguageChanged"}){
    var target=AccessTools.Method(VersionComponent,method);
    if(target==null)throw new MissingMethodException(VersionComponent.FullName,method);
    harmony.Patch(target,postfix:new HarmonyMethod(typeof(LanUi),"RefreshVersion"));
   }
  }
  public static void SetEnabled(bool enabled){
   Enabled=enabled;
   foreach(var instance in Payload.FindObjects(VersionComponent))RefreshVersion(instance);
   if(!enabled)Originals.Clear();
  }
  public static void RefreshVersion(object __instance){
   var label=AccessTools.Field(__instance.GetType(),"VersionText").GetValue(__instance);if(label==null)return;
   var textProperty=label.GetType().GetProperty("text");string text=(string)textProperty.GetValue(label,null);
   if(Enabled){
    if(String.IsNullOrEmpty(text))return;
    if(!text.EndsWith(LanCopy.VersionSuffix,StringComparison.Ordinal))Originals[label]=text;
    textProperty.SetValue(label,LanCopy.DecorateVersion(text),null);
   }else {string original;if(Originals.TryGetValue(label,out original))textProperty.SetValue(label,original,null);}
  }
 }
}
