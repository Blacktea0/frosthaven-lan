using System;
using System.Collections;
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using HarmonyLib;
using FrosthavenLan;
namespace ENet {
 public struct Address {}
 public class Host {
  public int Connect(Address address){return 1;}
  public int Connect(Address address,int count){return count;}
 }
}
namespace UnityEngine {
 public class RenderTexture {
  public static RenderTexture GetTemporary(int width,int height,int depth){return null;}
  public static void ReleaseTemporary(RenderTexture texture){}
 }
 public static class Graphics {public static void Blit(object source,RenderTexture target){}}
}
interface IBackend {}
class Eos:IBackend {public Eos(int manager,string token){}}
class Enet:IBackend {
 public Enet(int manager,string token){}
 public int RenamedConnection(){return new ENet.Host().Connect(new ENet.Address());}
}
class BadEnet:IBackend {public BadEnet(long manager,string token){}}
class UpdatedNetwork {
 internal static IBackend backend;
 static UpdatedNetwork(){throw new Exception("Discovery must not execute game initializers");}
 public static IEnumerator ACompletelyRenamedCoroutine(){yield return null;backend=new Eos(1,"token");}
 public static int Decoy(int value){switch(value){case 0:return 12;case 1:return 80;case 2:return 123;case 3:return 456;default:return 789;}}
}
class AmbiguousNetwork {
 internal static IBackend backend;
 public static void First(){backend=new Eos(1,"a");}
 public static void Second(){backend=new Eos(1,"b");}
}
class ChangedFlow {
 internal static IBackend backend;
 public static void Changed(){var candidate=new Eos(1,"x");GC.KeepAlive(candidate);backend=candidate;}
}
class AmbiguousConnections {
 public int First(){return new ENet.Host().Connect(new ENet.Address());}
 public int Second(){return new ENet.Host().Connect(new ENet.Address());}
}
class AlreadyTwoChannels {
 public int Changed(){return new ENet.Host().Connect(new ENet.Address(),2);}
}
class GatedNetwork {
 internal static IBackend backend;
 [MethodImpl(MethodImplOptions.NoInlining)] public static void Create(){backend=new Eos(1,"token");}
}
class InstallationGate {
 internal static bool Ready;
 public static bool IsReady(){return Ready;}
 public static IEnumerable<CodeInstruction> Rewrite(IEnumerable<CodeInstruction> code,ILGenerator generator){
  return PatchDiscovery.SwapBackend(code,typeof(Eos),typeof(Enet),typeof(GatedNetwork).GetField("backend",BindingFlags.NonPublic|BindingFlags.Static),typeof(InstallationGate).GetMethod("IsReady"),generator);
 }
}
class RenamedAvatarPlayer {
 public class RenamedState {
  public RenamedAvatarPlayer renamedOwner;
  public bool RenamedTick(){var rt=UnityEngine.RenderTexture.GetTemporary(128,128,0);UnityEngine.Graphics.Blit(null,rt);UnityEngine.RenderTexture.ReleaseTemporary(rt);GC.KeepAlive(renamedOwner);return true;}
 }
}
class AmbiguousAvatarPlayer {
 public class State {
  public bool First(){UnityEngine.Graphics.Blit(null,null);return true;}
  public bool Second(){UnityEngine.Graphics.Blit(null,null);return true;}
 }
}
class ChangedAvatarPlayer {
 public class State {
  public ChangedAvatarPlayer owner;
  public bool Tick(){var rt=UnityEngine.RenderTexture.GetTemporary(256,256,0);UnityEngine.Graphics.Blit(null,rt);UnityEngine.RenderTexture.ReleaseTemporary(rt);GC.KeepAlive(owner);return true;}
 }
}
class PatchDiscoveryTests {
 static FieldInfo Field(Type type){return type.GetField("backend",BindingFlags.NonPublic|BindingFlags.Static);}
 static void Require(bool value,string message){if(!value)throw new Exception(message);Console.WriteLine("PASS: "+message);}
 static void Refused(Action action,string message){try{action();}catch(NotSupportedException){Console.WriteLine("PASS: "+message);return;}throw new Exception("Expected refusal: "+message);}
 static int Main(){try{
  var init=PatchDiscovery.BackendInitialization(typeof(UpdatedNetwork),typeof(Eos),typeof(Enet),Field(typeof(UpdatedNetwork)));
  Require(init.Name=="MoveNext"&&init.DeclaringType.Name.Contains("ACompletelyRenamedCoroutine"),"renamed/reindexed backend coroutine is discovered by constructor and backend store");
  Require(PatchDiscovery.Replacement(PatchDiscovery.Read(init),typeof(Eos),typeof(Enet),Field(typeof(UpdatedNetwork))).DeclaringType==typeof(Enet),"matching backend constructor signature is validated without executing game code");
  Require(PatchDiscovery.Read(typeof(UpdatedNetwork).GetMethod("Decoy")).Any(i=>i.opcode==OpCodes.Switch),"IL reader handles switch operands without treating data as patch sites");
  Require(PatchDiscovery.Connect(typeof(Enet)).Name=="RenamedConnection","renamed ENet connection method is discovered by call structure");
  Refused(()=>PatchDiscovery.BackendInitialization(typeof(AmbiguousNetwork),typeof(Eos),typeof(Enet),Field(typeof(AmbiguousNetwork))),"multiple backend candidates are refused");
  Refused(()=>PatchDiscovery.BackendInitialization(typeof(UpdatedNetwork),typeof(Eos),typeof(BadEnet),Field(typeof(UpdatedNetwork))),"incompatible replacement constructor is refused");
  Refused(()=>PatchDiscovery.BackendInitialization(typeof(ChangedFlow),typeof(Eos),typeof(Enet),Field(typeof(ChangedFlow))),"changed backend assignment flow is refused");
  Refused(()=>PatchDiscovery.BackendInitialization(typeof(Enet),typeof(Eos),typeof(Enet),null),"missing backend initialization is refused");
  Refused(()=>PatchDiscovery.Connect(typeof(AmbiguousConnections)),"multiple ENet connection candidates are refused");
  Refused(()=>PatchDiscovery.Connect(typeof(AlreadyTwoChannels)),"changed ENet connection signature is refused");
  var avatar=PatchDiscovery.Avatar(typeof(RenamedAvatarPlayer));
  Require(avatar.Name=="RenamedTick"&&PatchDiscovery.AvatarOwner(avatar).Name=="renamedOwner","renamed avatar state method and owner field are discovered structurally");
  Refused(()=>PatchDiscovery.Avatar(typeof(AmbiguousAvatarPlayer)),"multiple avatar candidates are refused");
  Refused(()=>PatchDiscovery.Avatar(typeof(ChangedAvatarPlayer)),"changed avatar copy flow is refused");
  var harmony=new Harmony("frosthaven.tests.discovery.gate");
  harmony.Patch(typeof(GatedNetwork).GetMethod("Create"),transpiler:new HarmonyMethod(typeof(InstallationGate).GetMethod("Rewrite")));
  GatedNetwork.Create();Require(GatedNetwork.backend is Eos,"partial or failed installation retains original EOS construction");
  InstallationGate.Ready=true;GatedNetwork.Create();Require(GatedNetwork.backend is Enet,"completed installation enables the discovered ENet constructor");
  InstallationGate.Ready=false;GatedNetwork.Create();Require(GatedNetwork.backend is Eos,"deactivation immediately restores original constructor selection");
  harmony.UnpatchAll(harmony.Id);GatedNetwork.Create();Require(GatedNetwork.backend is Eos,"rollback restores the original method");
  return 0;
 }catch(Exception e){Console.Error.WriteLine(e);return 1;}}
}
