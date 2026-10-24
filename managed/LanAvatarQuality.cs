using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;
using System.Security.Cryptography;
using HarmonyLib;
namespace FrosthavenLan {
 public static class LanAvatarQuality {
  const BindingFlags All=BindingFlags.Public|BindingFlags.NonPublic|BindingFlags.Instance|BindingFlags.Static;
  public static void Install(Harmony harmony,Type player){
   var iterator=player.GetNestedTypes(All).Single(t=>t.Name.StartsWith("<StartLoadingAvatarCrt>"));
   harmony.Patch(AccessTools.Method(iterator,"MoveNext"),transpiler:new HarmonyMethod(typeof(LanAvatarQuality),"PreservePixels"));
  }
  static bool Calls(CodeInstruction i,string type,string name){var m=i.operand as MethodInfo;return m!=null&&m.DeclaringType.FullName==type&&m.Name==name;}
  public static IEnumerable<CodeInstruction> PreservePixels(IEnumerable<CodeInstruction> instructions,MethodBase __originalMethod){
   var code=instructions.ToList();
   var targets=code.Select((i,n)=>new {i,n}).Where(x=>Calls(x.i,"UnityEngine.RenderTexture","GetTemporary")).ToArray();
   if(targets.Length!=1)throw new Exception("Expected one avatar RenderTexture allocation");
   int start=targets[0].n-3;
   int end=code.FindIndex(targets[0].n,i=>Calls(i,"UnityEngine.RenderTexture","ReleaseTemporary"));
   if(start<0||end<0||code[start].opcode!=OpCodes.Ldc_I4||!Equals(code[start].operand,128)||code[start+1].opcode!=OpCodes.Ldc_I4||!Equals(code[start+1].operand,128)||code[start+2].opcode!=OpCodes.Ldc_I4_0)throw new Exception("Unsupported avatar resize block");
   var block=code.GetRange(start,end-start+1);
   if(block.Count(i=>Calls(i,"UnityEngine.Graphics","Blit"))!=1||block.Any(i=>i.blocks.Count!=0))throw new Exception("Unsupported avatar GPU copy flow");
   var owner=__originalMethod.DeclaringType.GetField("<>4__this",All);
   if(owner==null)throw new MissingFieldException(__originalMethod.DeclaringType.FullName,"<>4__this");
   var replacement=new CodeInstruction(OpCodes.Ldarg_0);
   replacement.labels.AddRange(block.SelectMany(i=>i.labels));
   code.RemoveRange(start,block.Count);
   code.InsertRange(start,new[]{replacement,new CodeInstruction(OpCodes.Ldfld,owner),new CodeInstruction(OpCodes.Call,AccessTools.Method(typeof(LanAvatarQuality),"CopyAvatar"))});
   return code;
  }
  internal static object Get(object instance,string name){var p=instance.GetType().GetProperty(name,All);return p!=null?p.GetValue(instance,null):instance.GetType().GetField(name,All).GetValue(instance);}
  // Read CPU mip 0, not a GPU sample affected by quality settings or mip selection.
  public static void CopyAvatar(object player){
   var avatar=Get(player,"Avatar");var source=Get(avatar,"texture");
   int width=(int)Get(source,"width"),height=(int)Get(source,"height");
   if(width<1||height<1||width>2048||height>2048)throw new Exception("Unsupported avatar source size: "+width+"x"+height);
   var pixels=(Array)source.GetType().GetMethod("GetPixels32",new[]{typeof(int)}).Invoke(source,new object[]{0});
   if(pixels.Length!=checked(width*height))throw new Exception("Avatar source pixel count mismatch");
   var textureType=Payload.Unity("UnityEngine.Texture2D");
   var format=Payload.Unity("UnityEngine.TextureFormat");
   object texture=null,sprite=null;
   try {
    texture=Activator.CreateInstance(textureType,new[]{(object)width,height,Enum.Parse(format,"RGBA32"),false});
    textureType.GetMethod("SetPixels32",new[]{pixels.GetType(),typeof(int)}).Invoke(texture,new object[]{pixels,0});
    textureType.GetMethod("Apply",new[]{typeof(bool),typeof(bool)}).Invoke(texture,new object[]{false,false});
    var ignore=textureType.GetProperty("ignoreMipmapLimit");if(ignore!=null)ignore.SetValue(texture,true,null);
    var rect=Activator.CreateInstance(Payload.Unity("UnityEngine.Rect"),new object[]{0f,0f,(float)width,(float)height});
    var pivot=Activator.CreateInstance(Payload.Unity("UnityEngine.Vector2"),new object[]{0.5f,0.5f});
    var spriteType=Payload.Unity("UnityEngine.Sprite");
    sprite=spriteType.GetMethod("Create",new[]{textureType,rect.GetType(),pivot.GetType()}).Invoke(null,new[]{texture,rect,pivot});
    player.GetType().GetProperty("Avatar").SetValue(player,sprite,null);
    // Reuse the game's existing Clear() ownership path to release our copy.
    player.GetType().GetField("_isAvatarCreatedFromStream",All).SetValue(player,true);
   } catch {
    var destroy=Payload.Unity("UnityEngine.Object").GetMethod("Destroy",new[]{Payload.Unity("UnityEngine.Object")});
    if(sprite!=null)destroy.Invoke(null,new[]{sprite});
    if(texture!=null)destroy.Invoke(null,new[]{texture});
    throw;
   }
   DiagnosticTrace.Write("Avatar CPU copy: "+width+"x"+height+"; source mipmaps="+Get(source,"mipmapCount")+"; source active mip="+Get(source,"activeMipmapLimit"));
  }
  public static string Describe(){
   var registry=Payload.Game.GetType("FFSNet.PlayerRegistry",true);
   var players=(System.Collections.IEnumerable)registry.GetField("AllPlayers",All).GetValue(null);
   var image=RuntimeAssemblies.Require("UnityEngine.ImageConversionModule").GetType("UnityEngine.ImageConversion",true);
   var result=new List<string>();
   foreach(var player in players){
    var sprite=Get(player,"Avatar");if(sprite==null)continue;
    var texture=Get(sprite,"texture");
    var png=(byte[])image.GetMethod("EncodeToPNG").Invoke(null,new[]{texture});
    string hash;using(var sha=SHA256.Create())hash=BitConverter.ToString(sha.ComputeHash(png)).Replace("-","");
    result.Add("AVATAR player="+Get(Get(player,"PlayerID"),"ID")+" local="+Object.ReferenceEquals(player,registry.GetProperty("MyPlayer",All).GetValue(null,null))+" size="+Get(texture,"width")+"x"+Get(texture,"height")+" mips="+Get(texture,"mipmapCount")+" activeMip="+Get(texture,"activeMipmapLimit")+" pngBytes="+png.Length+" sha256="+hash);
   }
   return String.Join("\n",result.ToArray());
  }
 }
}
