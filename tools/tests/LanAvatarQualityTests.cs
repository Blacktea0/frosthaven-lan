using System;
using System.Collections;
using System.Reflection;
using System.Runtime.CompilerServices;
using FrosthavenLan;
using HarmonyLib;
namespace UnityEngine {
 public class Object {public static void Destroy(Object obj){}}
 public enum TextureFormat {RGBA32}
 public struct Color32 {public byte r,g,b,a;}
 public struct Rect {public Rect(float x,float y,float w,float h){}}
 public struct Vector2 {public Vector2(float x,float y){}}
 public class Texture2D:Object {
  public int width{get;private set;}public int height{get;private set;}
  public int mipmapCount{get;private set;}public int activeMipmapLimit{get;set;}
  public bool ignoreMipmapLimit{get;set;}
  Color32[] pixels;
  public Texture2D(int w,int h,TextureFormat format,bool mipChain){width=w;height=h;mipmapCount=mipChain?8:1;pixels=new Color32[w*h];}
  public Color32[] GetPixels32(int mip){if(mip!=0)throw new Exception("Read incorrect mip");return (Color32[])pixels.Clone();}
  public void SetPixels32(Color32[] data,int mip){pixels=(Color32[])data.Clone();}
  public void Apply(bool updateMipmaps,bool makeNoLongerReadable){if(updateMipmaps||makeNoLongerReadable)throw new Exception("Unexpected mip generation or loss of readability");}
 }
 public class Sprite:Object {
  public Texture2D texture{get;private set;}
  public static Sprite Create(Texture2D t,Rect rect,Vector2 pivot){return new Sprite{texture=t};}
 }
 public class RenderTexture:Object {
  [MethodImpl(MethodImplOptions.NoInlining)] public static RenderTexture GetTemporary(int width,int height,int depth){return new RenderTexture();}
  public static void ReleaseTemporary(RenderTexture texture){}
 }
 public static class Graphics {
  public static int Calls;
  [MethodImpl(MethodImplOptions.NoInlining)] public static void Blit(Texture2D source,RenderTexture destination){Calls++;}
 }
}
namespace FrosthavenLan {
 public static class Payload {
  public static Assembly Game{get{return typeof(Payload).Assembly;}}
  public static Type Unity(string name){return Game.GetType(name,true);}
 }
 public static class RuntimeAssemblies {public static Assembly Require(string name){return typeof(Payload).Assembly;}}
 public static class DiagnosticTrace {public static void Write(string message){Console.WriteLine(message);}}
}
class FakeAvatarPlayer {
 public UnityEngine.Sprite Avatar{get;set;}
 bool _isAvatarCreatedFromStream;
 public bool OwnsCopy{get{return _isAvatarCreatedFromStream;}}
 public IEnumerator StartLoadingAvatarCrt(){
  yield return null;
  if(Avatar!=null){
   var rt=UnityEngine.RenderTexture.GetTemporary(128,128,0);
   UnityEngine.Graphics.Blit(Avatar.texture,rt);
   UnityEngine.RenderTexture.ReleaseTemporary(rt);
  }
  yield return null;
 }
}
class LanAvatarQualityTests {
 static void Require(bool value,string message){if(!value)throw new Exception(message);Console.WriteLine("PASS: "+message);}
 static void Run(FakeAvatarPlayer player){var routine=player.StartLoadingAvatarCrt();while(routine.MoveNext()){} }
 static int Main(){try{
  var source=new UnityEngine.Texture2D(184,184,UnityEngine.TextureFormat.RGBA32,true);source.activeMipmapLimit=3;
  var pixels=new UnityEngine.Color32[184*184];for(int i=0;i<pixels.Length;i++)pixels[i]=new UnityEngine.Color32{r=(byte)i,g=(byte)(i>>8),b=(byte)(i*7),a=(byte)(255-i%19)};
  source.SetPixels32(pixels,0);
  var original=UnityEngine.Sprite.Create(source,new UnityEngine.Rect(),new UnityEngine.Vector2());
  var player=new FakeAvatarPlayer{Avatar=original};
  var harmony=new Harmony("frosthaven.tests.avatarquality");
  LanAvatarQuality.Install(harmony,typeof(FakeAvatarPlayer));Run(player);
  var copy=player.Avatar.texture;
  Require(copy.width==184&&copy.height==184,"network avatar preserves actual source dimensions");
  Require(!Object.ReferenceEquals(copy,source)&&player.OwnsCopy,"copy has independent lifetime and uses game cleanup ownership");
  Require(copy.mipmapCount==1&&copy.ignoreMipmapLimit,"network copy disables mipmaps and ignores texture quality limits");
  Require(source.activeMipmapLimit==3&&source.mipmapCount==8,"source and global game quality are not modified");
  var actual=copy.GetPixels32(0);for(int i=0;i<pixels.Length;i++)if(actual[i].r!=pixels[i].r||actual[i].g!=pixels[i].g||actual[i].b!=pixels[i].b||actual[i].a!=pixels[i].a)throw new Exception("Pixel changed at "+i);
  Require(UnityEngine.Graphics.Calls==0,"original RGBA pixels bypass the GPU resize path");
  harmony.UnpatchAll(harmony.Id);Run(new FakeAvatarPlayer{Avatar=original});
  Require(UnityEngine.Graphics.Calls==1,"restore reinstates original GPU conversion");
  return 0;
 }catch(Exception e){Console.Error.WriteLine(e);return 1;}}
}
