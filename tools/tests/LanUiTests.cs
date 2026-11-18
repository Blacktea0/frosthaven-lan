using System;using System.Reflection;using System.Runtime.CompilerServices;using FrosthavenLan;using HarmonyLib;
public class FakeGameObject {
 public bool activeSelf {get;private set;}
 public void SetActive(bool active){activeSelf=active;}
}
public class FakeButton {
 public FakeGameObject gameObject {get;private set;}
 public FakeButton(){gameObject=new FakeGameObject();}
}
public class FakeVersionText {public string text {get;set;}}
public class VersionAndChangeLog {public FakeVersionText VersionText=new FakeVersionText();}
public interface INetworkSessionService {string GetInviteCode();}
public class UIMultiplayerEscSubmenu:INetworkSessionService {
 private readonly FakeVersionText inviteCodeText=new FakeVersionText();
 private readonly FakeButton copyCodeButton=new FakeButton();
 private readonly INetworkSessionService service;
 public UIMultiplayerEscSubmenu(){service=this;}
 public string SessionId="localhost";
 public string LocalizedButtonText;
 public string GetInviteCode(){return SessionId;}
 public string DisplayedCode {get{return inviteCodeText.text;}}
 public bool CopyVisible {get{return copyCodeButton.gameObject.activeSelf;}}
 [MethodImpl(MethodImplOptions.NoInlining)]
 public void LoadCurrentCode(){
  copyCodeButton.gameObject.SetActive(!String.IsNullOrEmpty(SessionId));
  if(CopyVisible)inviteCodeText.text=LocalizedButtonText??service.GetInviteCode();
 }
}
namespace I2.Loc {
 public static class LocalizationManager {
  public static string CurrentLanguageCode {get;set;}
  [MethodImpl(MethodImplOptions.NoInlining)]
  public static bool TryGetTranslation(string Term,out string Translation,string overrideLanguage){Translation="<color=#CDBF84>Session started!</color>\nOriginal body";return true;}
 }
}
namespace FrosthavenLan {
 public static class DiagnosticTrace {public static void Write(string message){Console.WriteLine(message);}}
 public static class Payload {
  public static object[] Instances=new object[0];
  public static Assembly Game {get{return typeof(VersionAndChangeLog).Assembly;}}
  public static object[] FindObjects(Type type){return Instances;}
 }
}
class LanUiTests {
 static void Require(bool value,string message){if(!value)throw new Exception(message);Console.WriteLine("PASS: "+message);}
 static int Main(){try{
  var menu=new VersionAndChangeLog();menu.VersionText.text="<size=90%>Retail 1.0.34121</size>";
  Payload.Instances=new object[]{menu};
  LanUi.SetEnabled(true);
  Require(menu.VersionText.text=="<size=90%>Retail 1.0.34121</size> LAN Enabled","active main menu displays LAN status without changing the version");
  LanUi.RefreshVersion(menu);LanUi.SetEnabled(true);
  Require(menu.VersionText.text.Split(new[]{"LAN Enabled"},StringSplitOptions.None).Length==2,"repeated enable and UI refresh do not duplicate the marker");
  menu.VersionText.text="版本 1.0.34121";LanUi.RefreshVersion(menu);
  Require(menu.VersionText.text=="版本 1.0.34121 LAN Enabled","language changes keep the marker");
  var reopened=new VersionAndChangeLog();reopened.VersionText.text="Retail 1.0.34121";LanUi.RefreshVersion(reopened);
  Require(reopened.VersionText.text.EndsWith(" LAN Enabled"),"newly opened menu components receive the marker");
  Payload.Instances=new object[]{menu,reopened};LanUi.SetEnabled(false);
  Require(menu.VersionText.text=="版本 1.0.34121"&&reopened.VersionText.text=="Retail 1.0.34121","restore removes markers and retains the latest localized original text");
  LanUi.RefreshVersion(menu);
  Require(menu.VersionText.text=="版本 1.0.34121","disabled patch leaves the original version label unchanged");
  LanText.Install(new Harmony("frosthaven.tests.host-invite"));
  var hostMenu=new UIMultiplayerEscSubmenu();
  hostMenu.LoadCurrentCode();
  Require(hostMenu.CopyVisible&&hostMenu.DisplayedCode=="localhost","disabled patch preserves the original copy button");
  LanUi.SetEnabled(true);
  I2.Loc.LocalizationManager.CurrentLanguageCode="zh-CN";
  hostMenu.LoadCurrentCode();
  Require(!hostMenu.CopyVisible&&hostMenu.DisplayedCode=="","host menu hides the copy control and clears the invite text");
  hostMenu.LocalizedButtonText="CONFIRM";
  hostMenu.LoadCurrentCode();
  Require(!hostMenu.CopyVisible&&hostMenu.DisplayedCode=="","LAN copy control stays hidden when the button relocalizes its text");
  string notification;
  I2.Loc.LocalizationManager.TryGetTranslation("GUI_NOTIFICATION_MULTIPLAYER_STARTED",out notification,null);
  Require(notification=="<color=#CDBF84>Session started!</color>\n将主机IP发给好友以邀请好友加入","the active localization patch changes the session-start notification body");
  hostMenu.LocalizedButtonText=null;
  hostMenu.SessionId="ABCDE123";
  hostMenu.LoadCurrentCode();
  Require(hostMenu.CopyVisible&&hostMenu.DisplayedCode=="ABCDE123","normal invite codes retain the copy control");
  return 0;
 }catch(Exception e){Console.Error.WriteLine(e);return 1;}}
}
