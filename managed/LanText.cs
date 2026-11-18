using System;using HarmonyLib;
namespace FrosthavenLan {
 public static class LanText {
  static Type Manager { get { return Payload.Game.GetType("I2.Loc.LocalizationManager",true); } }
  static Type HostMenu { get { return Payload.Game.GetType("UIMultiplayerEscSubmenu",true); } }
  static string LanguageCode(string language){return String.IsNullOrEmpty(language)?(string)Manager.GetProperty("CurrentLanguageCode").GetValue(null,null):(string)Manager.GetMethod("GetLanguageCode",new[]{typeof(string)}).Invoke(null,new object[]{language})??language;}
  public static void Translation(string Term,ref string Translation,string overrideLanguage,ref bool __result){
   if(!LanUi.Enabled||LanCopy.Index(Term)<0)return;
   string translated;if(!LanCopy.TryTranslate(Term,Translation,LanguageCode(overrideLanguage),out translated))return;
   Translation=translated;__result=true;
  }
  public static void HideHostInvite(object ___inviteCodeText,object ___copyCodeButton,object ___service){
   if(!LanUi.Enabled)return;
   var getCode=AccessTools.Method(Payload.Game.GetType("INetworkSessionService",true),"GetInviteCode",Type.EmptyTypes);
   if(!String.Equals((string)getCode.Invoke(___service,null),"localhost",StringComparison.Ordinal))return;
   var button=___copyCodeButton.GetType().GetProperty("gameObject").GetValue(___copyCodeButton,null);
   button.GetType().GetMethod("SetActive",new[]{typeof(bool)}).Invoke(button,new object[]{false});
   ___inviteCodeText.GetType().GetProperty("text").SetValue(___inviteCodeText,String.Empty,null);
  }
  public static void Install(Harmony harmony){
   harmony.Patch(AccessTools.Method(Manager,"TryGetTranslation"),postfix:new HarmonyMethod(typeof(LanText),"Translation"));
   var hostMenu=HostMenu;
   var loadCode=AccessTools.Method(hostMenu,"LoadCurrentCode",Type.EmptyTypes);
   if(loadCode==null)throw new MissingMethodException(hostMenu.FullName,"LoadCurrentCode");
   harmony.Patch(loadCode,postfix:new HarmonyMethod(typeof(LanText),"HideHostInvite"));
  }
  public static void Refresh(){
   foreach(var obj in Payload.FindObjects(Payload.Game.GetType("TextLocalizedListener",true))){try{AccessTools.Method(obj.GetType(),"OnLanguageChanged").Invoke(obj,null);}catch(Exception e){DiagnosticTrace.Write("Nonfatal text listener refresh failure: "+e);}}
   Manager.GetMethod("LocalizeAll",new[]{typeof(bool)}).Invoke(null,new object[]{true});
  }
 }
}
