using System;
using System.Linq;
using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using HarmonyLib;
namespace FrosthavenLan {
 public static class Payload {
  static Harmony harmony;
  static bool pendingSwitch=true;
  static volatile bool patchesReady;
  public static bool CanUseLanBackend(){return patchesReady;}
  static int frame;
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
   var backend=ctor.Invoke(new[]{Prop("Manager"),Prop("CurrentUserToken")});
   var handler=BackendProperty("ConnectRequestedHandler");handler.SetValue(backend,handler.GetValue(old,null),null);return backend;
  }
  static Type Net { get { return Game.GetType("FFSNetwork",true); } }
  const BindingFlags All=BindingFlags.Public|BindingFlags.NonPublic|BindingFlags.Static|BindingFlags.Instance;
  static object Prop(string n) { return Net.GetProperty(n,All).GetValue(null,null); }
  static FieldInfo Backend { get { return Net.GetField("_backend",All); } }
  static void ValidateNetworkContract(){
   var contract=Game.GetType("FFSNet.Backend.INetworkBackend",true);
   if(Backend==null||!Backend.IsStatic||Backend.FieldType!=contract||!contract.IsAssignableFrom(Enet)||!contract.IsAssignableFrom(Eos)||!typeof(IDisposable).IsAssignableFrom(Enet))throw new NotSupportedException("Unsupported network backend contract");
   var running=contract.GetProperty("IsRunning");var handler=contract.GetProperty("ConnectRequestedHandler");
   if(running==null||!running.CanRead||running.PropertyType!=typeof(bool)||handler==null||!handler.CanRead||!handler.CanWrite)throw new NotSupportedException("Unsupported network backend properties");
   foreach(string name in new[]{"Manager","CurrentUserToken","IsRunning"}){
    var property=Net.GetProperty(name,All);
    if(property==null||property.GetGetMethod(true)==null||!property.GetGetMethod(true).IsStatic||property.GetIndexParameters().Length!=0||(name=="IsRunning"&&property.PropertyType!=typeof(bool)))throw new NotSupportedException("Unsupported FFSNetwork property: "+name);
   }
  }
  public static void Install() {
   if(harmony!=null) return;
   try {
    DiagnosticTrace.Write("Waiting for Assembly-CSharp and UnityEngine.CoreModule");
    RuntimeAssemblies.WaitFor(new[]{"Assembly-CSharp","UnityEngine.CoreModule"},120000);
    ValidateNetworkContract();
    DiagnosticTrace.Write("Runtime assemblies loaded; discovering compatible patch sites; module="+Game.ManifestModule.ModuleVersionId);
    var init=PatchDiscovery.BackendInitialization(Net,Eos,Enet,Backend);
    var join=PatchDiscovery.Connect(Enet);
    var avatar=PatchDiscovery.Avatar(Game.GetType("FFSNet.NetworkPlayer",true));
    DiagnosticTrace.Write("Discovered backend initialization: "+PatchDiscovery.Describe(init));
    DiagnosticTrace.Write("Discovered ENet connection: "+PatchDiscovery.Describe(join));
    DiagnosticTrace.Write("Discovered avatar copy: "+PatchDiscovery.Describe(avatar));
    // Validate the actual Harmony instruction stream before changing any method.
    ILGenerator initGenerator;var initCode=PatchProcessor.GetOriginalInstructions(init,out initGenerator);
    SwapConstructor(initCode,initGenerator).ToArray();
    LanChannels.ConnectWithAvatarChannel(PatchProcessor.GetOriginalInstructions(join)).ToArray();
    LanAvatarQuality.PreservePixels(PatchProcessor.GetOriginalInstructions(avatar),avatar).ToArray();
    Type sync=Unity("UnityEngine.Object").Assembly.GetType("UnityEngine.UnitySynchronizationContext",true);
    var pump=AccessTools.Method(sync,"Exec",Type.EmptyTypes);
    if(pump==null||pump.IsStatic||pump.ReturnType!=typeof(void))throw new NotSupportedException("Unsupported Unity synchronization context");
    harmony=new Harmony("local.frosthaven.lan.prototype");
    harmony.Patch(init,transpiler:new HarmonyMethod(typeof(Payload),"SwapConstructor"));
    var ui=Game.GetType("GLOOM.MainMenu.UIMultiplayerJoinSessionWindow",true);
    harmony.Patch(AccessTools.Method(ui,"AwakeInit"),postfix:new HarmonyMethod(typeof(Payload),"InputPatch"));
    LanChannels.Install(harmony,join);
    LanAvatarQuality.Install(harmony,avatar);
    LanUi.Install(harmony);LanText.Install(harmony);
    // Install the pump last so a failed compatibility check cannot switch the
    // backend on the game thread while a partial patch set is being rolled back.
    harmony.Patch(pump,postfix:new HarmonyMethod(typeof(Payload),"Pump"));
    DiagnosticTrace.Write("Harmony installation complete; waiting for game-thread network initialization");
    patchesReady=true;
   } catch(Exception e){
    patchesReady=false;pendingSwitch=false;Session.Report("INSTALL ERROR "+e);
    if(harmony!=null){try{harmony.UnpatchAll(harmony.Id);}catch(Exception rollback){DiagnosticTrace.Write("Patch rollback failed: "+rollback);}harmony=null;}
   }
  }
  public static IEnumerable<CodeInstruction> SwapConstructor(IEnumerable<CodeInstruction> code,ILGenerator generator) {
   return PatchDiscovery.SwapBackend(code,Eos,Enet,Backend,AccessTools.Method(typeof(Payload),"CanUseLanBackend"),generator);
  }
  public static void InputPatch(object __instance){
   var f=AccessTools.Field(__instance.GetType(),"inviteCodeInput");
   var input=f==null?null:f.GetValue(__instance);
   if(input!=null){SetInput(input,"contentType","Standard");SetInput(input,"characterValidation","None");input.GetType().GetProperty("characterLimit").SetValue(input,253,null);}
  }
  static void SetInput(object input,string name,string value){var prop=input.GetType().GetProperty(name);prop.SetValue(input,Enum.Parse(prop.PropertyType,value),null);}
  public static void Pump(){
   if(!patchesReady)return;
   if(++frame%15!=0)return;
   try {
    if(pendingSwitch && Backend.GetValue(null)!=null && Prop("CurrentUserToken")!=null){
     var old=Backend.GetValue(null);
     if(Running(old)){Session.Report("REFUSED: active network session; pending switch cancelled");pendingSwitch=false;}
     else {
      DiagnosticTrace.Write("Game thread switching backend from "+old.GetType().FullName);
      if(!Enet.IsInstanceOfType(old)){
       var lan=NewBackend(Enet,old);
       originalBackend=old;Backend.SetValue(null,lan);
      }
      DiagnosticTrace.Write("Refreshing LAN version label, localized text and IP inputs");
      LanUi.SetEnabled(true);LanText.Refresh();pendingSwitch=false;
      foreach(var obj in FindObjects(Game.GetType("GLOOM.MainMenu.UIMultiplayerJoinSessionWindow")))InputPatch(obj);
      Session.Report("BACKEND FFSNet.Backend.EnetNetworkBackend");
     }
    }
    Session.Pump(Command);
   }catch(Exception e){Session.Report("PUMP ERROR "+e);}
  }
  static void Command(string cmd){
   if(cmd=="steam-status"){
    var asm=AppDomain.CurrentDomain.GetAssemblies().Single(a=>a.GetName().Name=="Facepunch.Steamworks.Win64");
    foreach(string typeName in new[]{"Steamworks.SteamClient","Steamworks.SteamUser","Steamworks.SteamUtils"}){
     var type=asm.GetType(typeName);if(type==null)continue;
     foreach(string name in new[]{"IsValid","IsLoggedOn","IsOverlayEnabled"}){var prop=type.GetProperty(name,BindingFlags.Public|BindingFlags.Static);if(prop!=null)Session.Report("STEAM_STATUS "+typeName+"."+name+"="+prop.GetValue(null,null));}
    }Session.Report("STEAM_STATUS_DONE");return;
   }
   if(cmd=="status"){Session.Report("STATUS backend="+Backend.GetValue(null)+" running="+Prop("IsRunning"));Session.Report(LanAvatarQuality.Describe());return;}
   if(cmd=="restore"){
    var current=Backend.GetValue(null);
    if(Running(current))throw new Exception("Cannot restore during active game session");
    // Early injection may select ENet before an original EOS instance exists.
    if(originalBackend==null && Enet.IsInstanceOfType(current)){
     var eos=NewBackend(Eos,current);
     originalBackend=eos;
    }
    if(originalBackend!=null){((IDisposable)current).Dispose();Backend.SetValue(null,originalBackend);}
    patchesReady=false;LanUi.SetEnabled(false);harmony.UnpatchAll(harmony.Id);LanText.Refresh();Session.Report("RESTORED; input settings revert on process exit");return;
   }
   throw new Exception("Unknown command");
  }
 }
}
