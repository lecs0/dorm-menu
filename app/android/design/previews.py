"""Static layout proposals using unchanged locally cached source text."""
from PIL import Image, ImageDraw, ImageFont
import json
from pathlib import Path
HERE=Path(__file__).parent
menu=json.loads((HERE/'../../../data/menus.json').read_text())['menus'][1]['days'][2]
def make(variant):
 im=Image.new('RGB',(1320,1500),'#E5E3DC'); d=ImageDraw.Draw(im)
 def text(x,y,s,size=17,c='#26352D',bold=False):
  f=ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans'+('-Bold' if bold else '')+'.ttf',size)
  d.text((x,y),s,font=f,fill=c)
 text(30,20,('A · Otthonos kártyák' if variant=='A' else 'B · Napi étlap'),30,bold=True)
 text(30,65,'Látványterv • Valódi menü: 2026. november 11. • A jelölés mintakulcsszava: burgonya',17)
 for n in range(3):
  x=30+n*430; dark=n==1; settings=n==2
  bg='#202824' if dark else '#FAF6ED'; card='#2C3630' if dark else '#FFFFFF'; fg='#F3F1E8' if dark else '#233C2F'; muted='#BDCABE' if dark else '#566458'; green='#B2CDB1' if dark else '#28533B'; accent='#594332' if dark else '#FBE3CB'
  d.rounded_rectangle((x,110,x+400,1450),radius=30,fill=bg)
  def t(a,b,s,size=17,bold=False,c=None):text(x+a,b,s,size,c or fg,bold)
  def box(y,h,fill=card):d.rounded_rectangle((x+16,y,x+384,y+h),radius=22 if variant=='A' else 10,fill=fill)
  t(24,130,'9:41',14);t(24,175,'Beállítások' if settings else 'Kollégiumi menü',25,True)
  if settings:
   t(24,222,'Személyre szabva, helyben mentve',16,c=muted)
   box(270,225);t(32,290,'Figyelt ételek',21,True);t(32,330,'Kulcsszavak, vesszővel elválasztva',15)
   d.rounded_rectangle((x+30,370,x+370,423),radius=12,outline=muted,width=1);t(42,384,'burgonya, gomba',18)
   t(32,447,'A találatokat az étlapon kiemeljük.',15,c=muted)
   box(515,265);t(32,538,'Holnapi emlékeztető',21,True);t(32,580,'Bekapcsolva',17)
   d.rounded_rectangle((x+300,576,x+360,610),radius=17,fill=green);d.ellipse((x+330,580,x+356,606),fill=bg)
   t(32,634,'Időpont                         18:00',19);t(32,678,'Értesítések engedélyezve',16,c=green)
   t(32,720,'Az Android késleltetheti a jelzést.',15,c=muted)
   box(800,140);t(32,822,'Kapcsolat és frissítés  ›',20,True);t(32,862,'koliapp.lecso.me',17);t(32,898,'Token biztonságosan mentve • • • •',14,c=muted)
   box(960,110);t(32,980,'Automatikus ellenőrzés',19,True);t(32,1020,'Megnyitáskor, 6 óránként',17)
   box(1090,56,green);t(123,1106,'Mentés',19,True,c=bg)
   t(32,1170,'Mentett menük törlése',17,c=green)
   t(24,1240,'A jelzés nem mondja le az étkezést.',15,c=muted)
  else:
   box(222,52);t(32,236,'46. hét · nov. 9–13.           ˅',18,True)
   for i,label in enumerate(['H','K','Sze','Cs','P']):
    xx=x+18+i*74
    d.rounded_rectangle((xx,294,xx+66,374),radius=20 if variant=='A' else 10,fill=green if i==2 else card)
    text(xx+15,305,label,16,bg if i==2 else fg,True);text(xx+22,333,str(9+i),18,bg if i==2 else fg)
    if i==2:text(xx+22,355,'MA',11,bg,True)
   t(24,402,'Ma, szerda',27 if variant=='A' else 31,True)
   t(24,447,'November 11.  •  Frissítés ↻',16,c=muted)
   y=491
   for i,m in enumerate(menu['meals']):
    lines=m['original_text'].splitlines();h=80+len(lines)*25
    box(y,h,accent if i==1 else card)
    if variant=='B':d.rounded_rectangle((x+16,y,x+22,y+h),radius=3,fill=green)
    t(32,y+16,['Reggeli','Ebéd','Vacsora'][i],21,True)
    yy=y+53
    for line in lines:t(32,yy,line,17,bold='burgonya' in line);yy+=25
    if i==1:t(32,yy+2,'Jelölt kulcsszó: burgonya',13,c=fg)
    y+=h+16
   t(24,y+6,'Frissítve: nov. 11. 18:30',14,c=muted)
   t(24,y+33,'Eredeti étlap (PDF) ↗',16,c=green)
   t(24,y+65,'Az allergénkódok a forrásból származnak.',12,c=muted)
   t(24,y+85,'Jelentésük és teljességük nem ellenőrzött.',12,c=muted)
  d.line((x+16,1350,x+384,1350),fill=muted,width=1)
  t(65,1382,'Étlap',18,not settings,c=green);t(220,1382,'Beállítások',18,settings,c=green)
 im.save(HERE/f'preview-{variant}.png')
for v in ['A','B']:make(v)
