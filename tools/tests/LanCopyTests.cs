using System;using System.IO;using System.Linq;using System.Text.RegularExpressions;using FrosthavenLan;
class LanCopyTests {
 static void Require(bool value,string label){if(!value)throw new Exception(label);Console.WriteLine("PASS: "+label);}
 static string Translate(string key,string language,string original="original") {string text;if(!LanCopy.TryTranslate(key,original,language,out text))throw new Exception("Unhandled term: "+key);return text;}
 static int Main(){try{
  string[] keys={"GUI_MULTIPLAYER_ENTER_INVITE_CODE","GUI_MULTIPLAYER_HOSTGAME_CONFIRMATION_DESC","GUI_MULTIPLAYER_INVITE_CODE","GUI_MULTIPLAYER_SESSION_MANAGER_COPYCODE","GUI_MULTIPLAYER_GENERATE_INVITE_CODE","GUI_MULTIPLAYER_JOIN_SESSION_DESCR","GUI_MULTIPLAYER_JOIN_SESSION_DESCRIPTION","GUI_MAIN_MENU_MULTIPLAYER_HOW_TO_HOST_DESCR","GUI_MAIN_MENU_MULTIPLAYER_TOOLTIP","GUI_MULTIPLAYER_CONNECTION_FAILED_InvalidCode"};
  foreach(string language in new[]{"en-GB","de","fr","it","es","pt-BR","hu","ru","zh-CN","zh-TW","ja","ko"}){
   foreach(string key in keys){string text=Translate("GUI/"+key,language);if(String.IsNullOrWhiteSpace(text)||text.Contains("邀请码")||text.IndexOf("invite code",StringComparison.OrdinalIgnoreCase)>=0)throw new Exception(language+" "+key);}
  }
  Console.WriteLine("PASS: all host-IP terms cover all 12 game languages");
  Require(Translate("GUI_MULTIPLAYER_INVITE_CODE","zh-CN")=="主机IP","Chinese host IP label");
  Require(Translate("GUI_MULTIPLAYER_INVITE_CODE","zh-Hant")=="主機IP","traditional Chinese language alias");
  Require(Translate("GUI_MULTIPLAYER_INVITE_CODE","unknown")=="Host IP","unknown languages fall back to English");
  string marker="<size=90%><sprite name=LeftKey></size> ";
  Require(Translate("GUI_MULTIPLAYER_SESSION_MANAGER_COPYCODE","zh-CN",marker+"Copy Code")==marker+"复制主机IP","copy control retains its controller glyph");
  Require(Translate("GUI_MAIN_MENU_MULTIPLAYER_TOOLTIP","en","<color=#F1DBAE>Multiplayer</color>\nJoin with an invite code.").StartsWith("<color=#F1DBAE>Multiplayer</color>\n"),"tooltip retains its styled heading");
  string unused;Require(!LanCopy.TryTranslate("GUI_MULTIPLAYER_HOST","Host","en",out unused)&&unused=="Host","unrelated localization remains unchanged");
  Require(LanCopy.DecorateVersion("Retail 1.0.34121")=="Retail 1.0.34121 LAN Enabled","version label preserves the actual game version");
  Require(LanCopy.DecorateVersion(LanCopy.DecorateVersion("Retail 1.0.34121"))=="Retail 1.0.34121 LAN Enabled","repeated refresh never duplicates the LAN marker");
  Require(LanCopy.DecorateVersion(null)==null&&LanCopy.DecorateVersion("")=="","uninitialized version labels remain empty");
  var dump=File.ReadAllText("docs/reverse-engineering/localization-dump.txt");int covered=0;
  foreach(Match m in Regex.Matches(dump,@"^([^\n\t]+)\t(.*?)(?=^[^\n\t]+\t|\z)",RegexOptions.Multiline|RegexOptions.Singleline)){
   string[] parts=m.Groups[2].Value.Split(new[]{" | "},StringSplitOptions.None);if(parts.Length<4)continue;
   if(Regex.IsMatch(parts[3],@"invite code|invalid code|copy code|generate new invite",RegexOptions.IgnoreCase)){
    Require(LanCopy.Index(m.Groups[1].Value)>=0,"dump coverage: "+m.Groups[1].Value);covered++;
   }
  }
  Require(covered>=9,"all invite-code terms in the analyzed game strings are covered");
  return 0;
 }catch(Exception e){Console.Error.WriteLine(e);return 1;}}
}
