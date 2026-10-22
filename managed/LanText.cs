using System;using System.Collections.Generic;using HarmonyLib;
namespace FrosthavenLan {
 public static class LanText {
  public const string Join="GUI_MULTIPLAYER_ENTER_INVITE_CODE", Host="GUI_MULTIPLAYER_HOSTGAME_CONFIRMATION_DESC";
  static readonly Dictionary<string,string[]> Text=new Dictionary<string,string[]>(StringComparer.OrdinalIgnoreCase){
   {"en",new[]{"Enter Host IP","Do you want to host a multiplayer LAN game?"}},
   {"de",new[]{"Host-IP eingeben","Möchtest du ein LAN-Mehrspielerspiel hosten?"}},
   {"fr",new[]{"Saisir l’adresse IP de l’hôte","Voulez-vous héberger une partie multijoueur en réseau local ?"}},
   {"it",new[]{"Inserisci l’IP dell’host","Vuoi ospitare una partita multiplayer in rete locale?"}},
   {"es",new[]{"Introduce la IP del anfitrión","¿Quieres alojar una partida multijugador en red local?"}},
   {"pt",new[]{"Insira o IP do anfitrião","Você deseja hospedar um jogo multiplayer em rede local?"}},
   {"hu",new[]{"Add meg a házigazda IP-címét","Szeretnél többjátékos játékot indítani helyi hálózaton?"}},
   {"ru",new[]{"Введите IP-адрес хоста","Хотите создать многопользовательскую игру по локальной сети?"}},
   {"zh-CN",new[]{"输入主机IP","你想要建立多人局域网游戏吗？"}},
   {"zh-TW",new[]{"輸入主機IP","你想要建立多人區域網路遊戲嗎？"}},
   {"ja",new[]{"ホストのIPアドレスを入力","LANでマルチプレイのゲームを作成しますか？"}},
   {"ko",new[]{"호스트 IP 입력","LAN 멀티플레이어 게임을 호스팅하시겠습니까?"}}
  };
  static Type Manager { get { return Payload.Game.GetType("I2.Loc.LocalizationManager",true); } }
  static string[] Resolve(string language){string code=String.IsNullOrEmpty(language)?(string)Manager.GetProperty("CurrentLanguageCode").GetValue(null,null):(string)Manager.GetMethod("GetLanguageCode",new[]{typeof(string)}).Invoke(null,new object[]{language});if(String.IsNullOrEmpty(code))code=language;string[] pair;if(code!=null&&Text.TryGetValue(code,out pair))return pair;string basic=(code??"en").Split('-')[0];return Text.TryGetValue(basic,out pair)?pair:Text["en"];}
  static int Index(string key){if(key==null)return -1;key=key.Substring(key.LastIndexOf('/')+1);return key==Join?0:key==Host?1:-1;}
  public static void Translation(string Term,ref string Translation,string overrideLanguage,ref bool __result){int i=Index(Term);if(i<0)return;Translation=Resolve(overrideLanguage)[i];__result=true;}
  public static void Install(Harmony harmony){harmony.Patch(AccessTools.Method(Manager,"TryGetTranslation"),postfix:new HarmonyMethod(typeof(LanText),"Translation"));}
  public static void Refresh(){foreach(var obj in Payload.FindObjects(Payload.Game.GetType("TextLocalizedListener",true))){try{AccessTools.Method(obj.GetType(),"OnLanguageChanged").Invoke(obj,null);}catch(Exception e){Payload.Log("TEXT_REFRESH "+e.Message);}}Manager.GetMethod("LocalizeAll",new[]{typeof(bool)}).Invoke(null,new object[]{true});}
 }
}
