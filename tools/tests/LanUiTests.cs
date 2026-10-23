using System;using System.Reflection;using FrosthavenLan;
public class FakeVersionText {public string text {get;set;}}
public class VersionAndChangeLog {public FakeVersionText VersionText=new FakeVersionText();}
namespace FrosthavenLan {
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
  return 0;
 }catch(Exception e){Console.Error.WriteLine(e);return 1;}}
}
