using System;
using System.Linq;
using System.Reflection.Emit;
using System.Runtime.CompilerServices;
using FrosthavenLan;
using HarmonyLib;
namespace ENet {
 public struct Address {}
 public class Host {
  [MethodImpl(MethodImplOptions.NoInlining)] public int Connect(Address address){return 1;}
  [MethodImpl(MethodImplOptions.NoInlining)] public int Connect(Address address,int channels){return channels;}
 }
}
class FakeBackend {
 [MethodImpl(MethodImplOptions.NoInlining)] public int JoinAsync(){return new ENet.Host().Connect(new ENet.Address());}
}
class LanChannelsTests {
 static void Require(bool value,string message){if(!value)throw new Exception(message);Console.WriteLine("PASS: "+message);}
 static int Main(){try{
  var harmony=new Harmony("frosthaven.tests.channels");
  LanChannels.Install(harmony,typeof(FakeBackend));
  Require(new FakeBackend().JoinAsync()==2,"patched LAN join negotiates both main and avatar channels");
  Require(new ENet.Host().Connect(new ENet.Address())==1,"unrelated ENet callers retain their original configuration");
  harmony.UnpatchAll(harmony.Id);
  Require(new FakeBackend().JoinAsync()==1,"restore removes the channel patch");
  bool refused=false;
  try{LanChannels.ConnectWithAvatarChannel(new[]{new CodeInstruction(OpCodes.Ret)}).ToArray();}catch(Exception){refused=true;}
  Require(refused,"unexpected game method shape fails patch installation");
  return 0;
 }catch(Exception e){Console.Error.WriteLine(e);return 1;}}
}
