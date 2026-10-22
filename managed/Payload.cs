using System;
using System.IO;
using System.Linq;
using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using System.Threading;
using HarmonyLib;
namespace FrosthavenLan {
 public static class Payload {
  static readonly string Dir=Path.GetDirectoryName(typeof(Payload).Assembly.Location);
  static Harmony harmony;
  static bool pendingSwitch=true;
  static int frame; static readonly object LogLock=new object();
  static object originalBackend;
  internal static Assembly Game { get { return RuntimeAssemblies.Require("Assembly-CSharp"); } }
  static Type Enet { get { return Game.GetType("FFSNet.Backend.EnetNetworkBackend",true); } }
  static Type Eos { get { return Game.GetType("FFSNet.Backend.EOSNetworkBackend",true); } }
  internal static Type Unity(string name) { return RuntimeAssemblies.Require("UnityEngine.CoreModule").GetType(name,true); }
  internal static object[] FindObjects(Type type) { return (object[])Unity("UnityEngine.Resources").GetMethod("FindObjectsOfTypeAll",new[]{typeof(Type)}).Invoke(null,new object[]{type}); }
  static PropertyInfo BackendProperty(string name) { return Game.GetType("FFSNet.Backend.INetworkBackend",true).GetProperty(name); }
  static bool Running(object backend) { return (bool)BackendProperty("IsRunning").GetValue(backend,null); }
  static object NewBackend(Type type,object old) {
   var ctor=type.GetConstructors().Single(c=>{var p=c.GetParameters();return p.Length==2 && p[0].ParameterType.IsInstanceOfType(Prop("Manager")) && p[1].ParameterType.IsInstanceOfType(Prop("CurrentUserToken"));});
   if(ctor==null)throw new MissingMethodException(type.FullName,".ctor(NetworkManager, UserToken)");
   var backend=ctor.Invoke(new[]{Prop("Manager"),Prop("CurrentUserToken")});
   var handler=BackendProperty("ConnectRequestedHandler");handler.SetValue(backend,handler.GetValue(old,null),null);return backend;
  }
  static Type Net { get { return Game.GetType("FFSNetwork",true); } }
  const BindingFlags All=BindingFlags.Public|BindingFlags.NonPublic|BindingFlags.Static|BindingFlags.Instance;
  static object Prop(string n) { return Net.GetProperty(n,All).GetValue(null,null); }
  static FieldInfo Backend { get { return Net.GetField("_backend",All); } }
  public static void Log(string s){lock(LogLock){using(var fs=new FileStream(Path.Combine(Dir,"payload.log"),FileMode.Append,FileAccess.Write,FileShare.ReadWrite|FileShare.Delete))using(var writer=new StreamWriter(fs)){writer.WriteLine(DateTime.UtcNow.ToString("o")+" "+s);}}}
  public static void Install() {
   if(harmony!=null) return;
   try {
    RuntimeAssemblies.WaitFor(new[]{"Assembly-CSharp","UnityEngine.CoreModule"},120000,Log);
    harmony=new Harmony("local.frosthaven.lan.prototype");
    Type sync=Unity("UnityEngine.Object").Assembly.GetType("UnityEngine.UnitySynchronizationContext",true);
    harmony.Patch(AccessTools.Method(sync,"Exec"),postfix:new HarmonyMethod(typeof(Payload),"Pump"));
    var init=Net.GetNestedTypes(All).Single(t=>t.Name.StartsWith("<InitializeCrt>"));
    harmony.Patch(AccessTools.Method(init,"MoveNext"),transpiler:new HarmonyMethod(typeof(Payload),"SwapConstructor"));
    var ui=Game.GetType("GLOOM.MainMenu.UIMultiplayerJoinSessionWindow",true);
    harmony.Patch(AccessTools.Method(ui,"AwakeInit"),postfix:new HarmonyMethod(typeof(Payload),"InputPatch"));
    LanText.Install(harmony);Log("INSTALLED patches; awaiting main thread");
   } catch(Exception e){Log("INSTALL ERROR "+e);if(harmony!=null){harmony.UnpatchAll(harmony.Id);harmony=null;}}
  }
  public static IEnumerable<CodeInstruction> SwapConstructor(IEnumerable<CodeInstruction> code) {
   int count=0;
   foreach(var i in code){var c=i.operand as ConstructorInfo;
    if(i.opcode==OpCodes.Newobj && c!=null && c.DeclaringType==Eos) {
     i.operand=Enet.GetConstructor(c.GetParameters().Select(p=>p.ParameterType).ToArray());count++;
    } yield return i;
   } if(count!=1)throw new Exception("Expected exactly one EOS constructor, found "+count);
  }
  public static void InputPatch(object __instance){
   var f=AccessTools.Field(__instance.GetType(),"inviteCodeInput");
   var input=f==null?null:f.GetValue(__instance);
   if(input!=null){SetInput(input,"contentType","Standard");SetInput(input,"characterValidation","None");input.GetType().GetProperty("characterLimit").SetValue(input,253,null);Log("IP input enabled");}
  }
  static void SetInput(object input,string name,string value){var prop=input.GetType().GetProperty(name);prop.SetValue(input,Enum.Parse(prop.PropertyType,value),null);}
  public static void Pump(){
   if(++frame%15!=0)return;
   try {
    if(pendingSwitch && Backend.GetValue(null)!=null && Prop("CurrentUserToken")!=null){
     var old=Backend.GetValue(null);
     if(Running(old)){Log("REFUSED: active network session; pending switch cancelled");pendingSwitch=false;}
     else {
      if(!Enet.IsInstanceOfType(old)){
       var lan=NewBackend(Enet,old);
       originalBackend=old;Backend.SetValue(null,lan);
      }
      LanText.Refresh();pendingSwitch=false;Log("BACKEND "+Backend.GetValue(null).GetType().FullName+" main-thread="+Thread.CurrentThread.ManagedThreadId);
      foreach(var obj in FindObjects(Game.GetType("GLOOM.MainMenu.UIMultiplayerJoinSessionWindow")))InputPatch(obj);
     }
    }
    string command=Path.Combine(Dir,"command.txt");
    if(File.Exists(command)){string s;try{s=File.ReadAllText(command).Trim();File.Delete(command);}catch(IOException){return;}Command(s);}
   }catch(Exception e){Log("PUMP ERROR "+e);}
  }
  static void Command(string cmd){
   Log("COMMAND "+cmd);
   if(cmd=="steam-status"){
    var asm=AppDomain.CurrentDomain.GetAssemblies().Single(a=>a.GetName().Name=="Facepunch.Steamworks.Win64");
    foreach(string typeName in new[]{"Steamworks.SteamClient","Steamworks.SteamUser","Steamworks.SteamUtils"}){
     var type=asm.GetType(typeName);if(type==null)continue;
     foreach(string name in new[]{"IsValid","IsLoggedOn","IsOverlayEnabled"}){var prop=type.GetProperty(name,BindingFlags.Public|BindingFlags.Static);if(prop!=null)Log("STEAM_STATUS "+typeName+"."+name+"="+prop.GetValue(null,null));}
    }Log("STEAM_STATUS_DONE");return;
   }
   if(cmd=="status"){Log("STATUS backend="+Backend.GetValue(null)+" running="+Prop("IsRunning"));return;}
   if(cmd=="restore"){
    var current=Backend.GetValue(null);
    if(Running(current))throw new Exception("Cannot restore during active game session");
    // Early injection may select ENet before an original EOS instance exists.
    if(originalBackend==null && Enet.IsInstanceOfType(current)){
     var eos=NewBackend(Eos,current);
     originalBackend=eos;
    }
    if(originalBackend!=null){((IDisposable)current).Dispose();Backend.SetValue(null,originalBackend);}
    harmony.UnpatchAll(harmony.Id);LanText.Refresh();Log("RESTORED; input settings revert on process exit");return;
   }
   throw new Exception("Unknown command");
  }
 }
}
