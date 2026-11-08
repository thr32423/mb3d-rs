object TextureFrm: TTextureFrm
  Left = 260
  Top = 200
  BorderStyle = bsDialog
  Caption = '@MB:TextureExport2'
  ClientHeight = 352
  ClientWidth = 520
  Color = clBtnFace
  Font.Charset = DEFAULT_CHARSET
  Font.Color = clWindowText
  Font.Height = -11
  Font.Name = 'MS Sans Serif'
  Font.Style = []
  OldCreateOrder = False
  Position = poScreenCenter
  OnShow = FormShow
  PixelsPerInch = 96
  TextHeight = 13
  object MapsLabel: TLabel
    Left = 12
    Top = 12
    Width = 300
    Height = 13
    Caption = '@MB:MapsOfTheCalculatedPicture'
  end
  object ColorCheck: TCheckBox
    Left = 24
    Top = 32
    Width = 300
    Height = 17
    Caption = '@MB:Color'
    Checked = True
    State = cbChecked
    TabOrder = 0
  end
  object HeightCheck: TCheckBox
    Left = 24
    Top = 52
    Width = 300
    Height = 17
    Caption = '@MB:Height16BitNearestPointWhite'
    Checked = True
    State = cbChecked
    TabOrder = 1
  end
  object NormalCheck: TCheckBox
    Left = 24
    Top = 72
    Width = 300
    Height = 17
    Caption = '@MB:NormalMap'
    Checked = True
    State = cbChecked
    TabOrder = 2
  end
  object AoCheck: TCheckBox
    Left = 24
    Top = 92
    Width = 300
    Height = 17
    Caption = '@MB:AmbientOcclusion16Bit'
    Checked = True
    State = cbChecked
    TabOrder = 3
  end
  object TileCheck: TCheckBox
    Left = 24
    Top = 116
    Width = 480
    Height = 17
    Caption = '@MB:TileableMirroredToTwiceTheSize'
    TabOrder = 4
  end
  object CameraBtn: TButton
    Left = 340
    Top = 28
    Width = 168
    Height = 25
    Hint = '@MB:APlanarOpticWithAField'
    Caption = '@MB:CameraForTextures'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 5
    OnClick = CameraBtnClick
  end
  object NameLabel: TLabel
    Left = 12
    Top = 150
    Width = 60
    Height = 13
    Caption = '@MB:Name'
  end
  object NameEdit: TEdit
    Left = 80
    Top = 146
    Width = 428
    Height = 21
    TabOrder = 6
  end
  object FolderLabel: TLabel
    Left = 12
    Top = 178
    Width = 60
    Height = 13
    Caption = '@MB:Folder2'
  end
  object FolderEdit: TEdit
    Left = 80
    Top = 174
    Width = 392
    Height = 21
    TabOrder = 7
  end
  object FolderBtn: TButton
    Left = 478
    Top = 173
    Width = 30
    Height = 23
    Caption = '...'
    TabOrder = 8
    OnClick = FolderBtnClick
  end
  object NoteLabel: TLabel
    Left = 12
    Top = 206
    Width = 496
    Height = 56
    AutoSize = False
    WordWrap = True
    Caption = '@MB:EachMapIsAPNGOf'
  end
  object WriteBtn: TButton
    Left = 12
    Top = 272
    Width = 160
    Height = 25
    Caption = '@MB:WriteTheMaps'
    TabOrder = 9
    OnClick = WriteBtnClick
  end
  object OpenBtn: TButton
    Left = 180
    Top = 272
    Width = 140
    Height = 25
    Caption = '@MB:OpenTheFolder'
    Enabled = False
    TabOrder = 10
    OnClick = OpenBtnClick
  end
  object StatusLabel: TLabel
    Left = 12
    Top = 306
    Width = 496
    Height = 40
    AutoSize = False
    WordWrap = True
    Caption = ''
  end
  object CloseBtn: TButton
    Left = 418
    Top = 272
    Width = 90
    Height = 25
    Caption = '@MB:Close'
    TabOrder = 11
    OnClick = CloseBtnClick
  end
end
