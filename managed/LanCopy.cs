using System;using System.Collections.Generic;using System.Text.RegularExpressions;
namespace FrosthavenLan {
 public static class LanCopy {
  public const string VersionSuffix=" LAN Enabled";
  static readonly Dictionary<string,string[]> Text=new Dictionary<string,string[]>(StringComparer.OrdinalIgnoreCase){
   {"en",new[]{"Enter Host IP","Do you want to host a multiplayer LAN game?","Host IP","Copy Host IP","Use Host IP","Get the host’s IPv4 address, enter it below without a port, and confirm to join the LAN game.","Load a campaign, open the pause menu and select Multiplayer to host a session. Share the host’s IPv4 address with other players. Assign characters after they join, then confirm. Players can join on the world map or during card selection.","Join a multiplayer LAN game using the host’s IP address.","Invalid host IP"}},
   {"de",new[]{"Host-IP eingeben","Möchtest du ein LAN-Mehrspielerspiel hosten?","Host-IP","Host-IP kopieren","Host-IP verwenden","Frage den Host nach seiner IPv4-Adresse, gib sie unten ohne Port ein und bestätige, um dem LAN-Spiel beizutreten.","Lade eine Kampagne und öffne im Pausenmenü Mehrspieler, um eine Sitzung zu hosten. Teile die IPv4-Adresse des Hosts. Weise nach dem Beitritt Charaktere zu und bestätige. Beitritt ist auf der Weltkarte oder bei der Kartenauswahl möglich.","Mit der IP-Adresse des Hosts einem LAN-Spiel beitreten.","Ungültige Host-IP"}},
   {"fr",new[]{"Saisir l’adresse IP de l’hôte","Voulez-vous héberger une partie multijoueur en réseau local ?","IP de l’hôte","Copier l’IP de l’hôte","Utiliser l’IP de l’hôte","Demandez l’adresse IPv4 de l’hôte, saisissez-la ci-dessous sans port et confirmez pour rejoindre la partie en réseau local.","Chargez une campagne et choisissez Multijoueur dans le menu pause pour héberger une partie. Partagez l’adresse IPv4 de l’hôte. Attribuez les personnages après la connexion des joueurs, puis confirmez. Ils peuvent rejoindre sur la carte du monde ou pendant le choix des cartes.","Rejoignez une partie en réseau local avec l’adresse IP de l’hôte.","IP de l’hôte invalide"}},
   {"it",new[]{"Inserisci l’IP dell’host","Vuoi ospitare una partita multiplayer in rete locale?","IP dell’host","Copia IP dell’host","Usa IP dell’host","Chiedi l’indirizzo IPv4 dell’host, inseriscilo qui sotto senza porta e conferma per unirti alla partita LAN.","Carica una campagna e seleziona Multigiocatore nel menu di pausa per ospitare una partita. Condividi l’indirizzo IPv4 dell’host. Assegna i personaggi ai giocatori connessi e conferma. È possibile unirsi dalla mappa del mondo o durante la scelta delle carte.","Unisciti a una partita LAN usando l’indirizzo IP dell’host.","IP dell’host non valido"}},
   {"es",new[]{"Introduce la IP del anfitrión","¿Quieres alojar una partida multijugador en red local?","IP del anfitrión","Copiar IP del anfitrión","Usar IP del anfitrión","Pide la dirección IPv4 del anfitrión, introdúcela abajo sin puerto y confirma para unirte a la partida LAN.","Carga una campaña y selecciona Multijugador en el menú de pausa para alojar una partida. Comparte la dirección IPv4 del anfitrión. Asigna personajes a los jugadores que se unan y confirma. Pueden unirse desde el mapa del mundo o durante la selección de cartas.","Únete a una partida LAN usando la dirección IP del anfitrión.","IP del anfitrión no válida"}},
   {"pt",new[]{"Insira o IP do anfitrião","Você deseja hospedar um jogo multiplayer em rede local?","IP do anfitrião","Copiar IP do anfitrião","Usar IP do anfitrião","Peça o endereço IPv4 do anfitrião, insira-o abaixo sem porta e confirme para entrar na partida LAN.","Carregue uma campanha e selecione Multijogador no menu de pausa para hospedar uma partida. Compartilhe o endereço IPv4 do anfitrião. Atribua personagens aos jogadores conectados e confirme. Eles podem entrar pelo mapa do mundo ou durante a seleção de cartas.","Entre em uma partida LAN usando o endereço IP do anfitrião.","IP do anfitrião inválido"}},
   {"hu",new[]{"Add meg a házigazda IP-címét","Szeretnél többjátékos játékot indítani helyi hálózaton?","Házigazda IP-címe","Házigazda IP-címének másolása","Házigazda IP-címének használata","Kérd el a házigazda IPv4-címét, írd be alább port nélkül, majd erősítsd meg a LAN-játékhoz való csatlakozást.","Tölts be egy hadjáratot, majd a szünet menü Többjátékos pontjában indíts játékot. Oszd meg a házigazda IPv4-címét. A csatlakozókhoz rendelj karaktereket és erősítsd meg. Csatlakozni a világtérképen vagy kártyaválasztáskor lehet.","Csatlakozz LAN-játékhoz a házigazda IP-címével.","Érvénytelen házigazda-IP"}},
   {"ru",new[]{"Введите IP-адрес хоста","Хотите создать многопользовательскую игру по локальной сети?","IP-адрес хоста","Копировать IP хоста","Использовать IP хоста","Узнайте IPv4-адрес хоста, введите его ниже без порта и подтвердите подключение к игре по локальной сети.","Загрузите кампанию и выберите сетевую игру в меню паузы, чтобы создать сессию. Передайте игрокам IPv4-адрес хоста. После подключения назначьте персонажей и подтвердите. Подключаться можно на карте мира или при выборе карт.","Подключитесь к игре по локальной сети с помощью IP-адреса хоста.","Неверный IP-адрес хоста"}},
   {"zh-CN",new[]{"输入主机IP","你想要建立多人局域网游戏吗？","主机IP","复制主机IP","使用主机IP","向房主获取主机的 IPv4 地址，在下方输入主机IP（不附加端口），然后确认加入局域网游戏。","载入战役，在暂停菜单中选择“多人游戏”并创建会话。将主机的 IPv4 地址告知其他玩家；玩家加入后分配角色并确认。玩家可在世界地图或战斗选牌阶段加入。","使用主机IP加入多人局域网游戏。","主机IP无效"}},
   {"zh-TW",new[]{"輸入主機IP","你想要建立多人區域網路遊戲嗎？","主機IP","複製主機IP","使用主機IP","向房主取得主機的 IPv4 位址，在下方輸入主機IP（不附加連接埠），然後確認加入區域網路遊戲。","載入戰役，在暫停選單中選擇「多人遊戲」並建立連線。將主機的 IPv4 位址告知其他玩家；玩家加入後分配角色並確認。玩家可在世界地圖或戰鬥選牌階段加入。","使用主機IP加入多人區域網路遊戲。","主機IP無效"}},
   {"ja",new[]{"ホストのIPアドレスを入力","LANでマルチプレイのゲームを作成しますか？","ホストIP","ホストIPをコピー","ホストIPを使用","ホストのIPv4アドレスを確認し、ポート番号を付けずに下の欄に入力して、LANゲームへの参加を確定してください。","キャンペーンを読み込み、ポーズメニューのマルチプレイからゲームを作成します。ホストのIPv4アドレスを共有し、参加したプレイヤーにキャラクターを割り当てて確定します。ワールドマップまたはカード選択中に参加できます。","ホストのIPアドレスを使ってLANゲームに参加します。","ホストIPが無効です"}},
   {"ko",new[]{"호스트 IP 입력","LAN 멀티플레이어 게임을 호스팅하시겠습니까?","호스트 IP","호스트 IP 복사","호스트 IP 사용","호스트의 IPv4 주소를 받아 포트 번호 없이 아래에 입력하고 확인하여 LAN 게임에 참여하세요.","캠페인을 불러오고 일시 정지 메뉴에서 멀티플레이어를 선택하여 게임을 만드세요. 호스트의 IPv4 주소를 공유하고 참여한 플레이어에게 캐릭터를 배정한 뒤 확인하세요. 세계 지도 또는 카드 선택 단계에서 참여할 수 있습니다.","호스트 IP 주소로 LAN 게임에 참여합니다.","잘못된 호스트 IP"}},
  };
  public static string DecorateVersion(string text){return String.IsNullOrEmpty(text)||text.EndsWith(VersionSuffix,StringComparison.Ordinal)?text:text+VersionSuffix;}
  static string[] Resolve(string code){
   code=code??"en";string[] row;
   if(Text.TryGetValue(code,out row))return row;
   if(code.StartsWith("zh",StringComparison.OrdinalIgnoreCase))return Text[code.IndexOf("Hant",StringComparison.OrdinalIgnoreCase)>=0||code.EndsWith("HK",StringComparison.OrdinalIgnoreCase)||code.EndsWith("MO",StringComparison.OrdinalIgnoreCase)||code.EndsWith("CHT",StringComparison.OrdinalIgnoreCase)?"zh-TW":"zh-CN"];
   return Text.TryGetValue(code.Split('-')[0],out row)?row:Text["en"];
  }
  public static int Index(string key){
   if(key==null)return -1;key=key.Substring(key.LastIndexOf('/')+1);
   switch(key){
    case "GUI_MULTIPLAYER_ENTER_INVITE_CODE":return 0;
    case "GUI_MULTIPLAYER_HOSTGAME_CONFIRMATION_DESC":return 1;
    case "GUI_MULTIPLAYER_INVITE_CODE":return 2;
    case "GUI_MULTIPLAYER_SESSION_MANAGER_COPYCODE":return 3;
    case "GUI_MULTIPLAYER_GENERATE_INVITE_CODE":return 4;
    case "GUI_MULTIPLAYER_JOIN_SESSION_DESCR":case "GUI_MULTIPLAYER_JOIN_SESSION_DESCRIPTION":return 5;
    case "GUI_MAIN_MENU_MULTIPLAYER_HOW_TO_HOST_DESCR":return 6;
    case "GUI_MAIN_MENU_MULTIPLAYER_TOOLTIP":return 7;
    case "GUI_MULTIPLAYER_CONNECTION_FAILED_InvalidCode":return 8;
    default:return -1;
   }
  }
  public static bool TryTranslate(string key,string original,string code,out string translation){
   int index=Index(key);translation=original;if(index<0)return false;
   translation=Resolve(code)[index];
   if(index==3&&!String.IsNullOrEmpty(original))translation=Regex.Match(original,@"^(?:\s*<[^>]+>)*\s*").Value+translation;
   if(index==7&&!String.IsNullOrEmpty(original)){int newline=original.IndexOf('\n');if(newline>=0)translation=original.Substring(0,newline+1)+translation;}
   return true;
  }
 }
}
