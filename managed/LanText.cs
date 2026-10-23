using System;using HarmonyLib;
namespace FrosthavenLan {
 public static class LanText {
  static Type Manager { get { return Payload.Game.GetType("I2.Loc.LocalizationManager",true); } }
  static string LanguageCode(string language){return String.IsNullOrEmpty(language)?(string)Manager.GetProperty("CurrentLanguageCode").GetValue(null,null):(string)Manager.GetMethod("GetLanguageCode",new[]{typeof(string)}).Invoke(null,new object[]{language})??language;}
  public static void Translation(string Term,ref string Translation,string overrideLanguage,ref bool __result){
   if(!LanUi.Enabled||LanCopy.Index(Term)<0)return;
   string translated;if(!LanCopy.TryTranslate(Term,Translation,LanguageCode(overrideLanguage),out translated))return;
   Translation=translated;__result=true;
  }
  public static void Install(Harmony harmony){harmony.Patch(AccessTools.Method(Manager,"TryGetTranslation"),postfix:new HarmonyMethod(typeof(LanText),"Translation"));}
  public static void Refresh(){foreach(var obj in Payload.FindObjects(Payload.Game.GetType("TextLocalizedListener",true))){try{AccessTools.Method(obj.GetType(),"OnLanguageChanged").Invoke(obj,null);}catch(Exception){/* One stale listener must not prevent other UI text from refreshing. */}}Manager.GetMethod("LocalizeAll",new[]{typeof(bool)}).Invoke(null,new object[]{true});}
 }
}
