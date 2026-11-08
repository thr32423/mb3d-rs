object ShareFrm: TShareFrm
  Left = 220
  Top = 160
  BorderStyle = bsDialog
  Caption = '@MB:Share'
  ClientHeight = 452
  ClientWidth = 540
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
  object Label1: TLabel
    Left = 8
    Top = 12
    Width = 80
    Height = 13
    Caption = '@MB:Title'
  end
  object TitleEdit: TEdit
    Left = 96
    Top = 8
    Width = 436
    Height = 21
    TabOrder = 0
    OnChange = FieldChange
  end
  object Label2: TLabel
    Left = 8
    Top = 40
    Width = 80
    Height = 13
    Caption = '@MB:Artist'
  end
  object ArtistEdit: TEdit
    Left = 96
    Top = 36
    Width = 436
    Height = 21
    TabOrder = 1
    OnChange = FieldChange
  end
  object Label3: TLabel
    Left = 8
    Top = 68
    Width = 80
    Height = 13
    Caption = '@MB:Credit'
  end
  object CreditEdit: TEdit
    Left = 96
    Top = 64
    Width = 436
    Height = 21
    Hint = '@MB:WhoseParametersYouStartedFromGroups'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 2
    OnChange = FieldChange
  end
  object PicCheck: TCheckBox
    Left = 8
    Top = 98
    Width = 140
    Height = 17
    Caption = '@MB:PictureJPEG'
    Checked = True
    State = cbChecked
    TabOrder = 3
    OnClick = FieldChange
  end
  object SizeCombo: TComboBox
    Left = 160
    Top = 96
    Width = 270
    Height = 21
    Style = csDropDownList
    ItemHeight = 13
    ItemIndex = 0
    TabOrder = 4
    Items.Strings = (
      '@MB:FacebookSize2048Px360Photos'
      '@MB:FullSize')
  end
  object ParamCheck: TCheckBox
    Left = 8
    Top = 124
    Width = 420
    Height = 17
    Hint = '@MB:FacebookTurnsParametersPastedIntoA'
    Caption = '@MB:ParametersAsATextFileTxt'
    Checked = True
    ParentShowHint = False
    ShowHint = True
    State = cbChecked
    TabOrder = 5
    OnClick = FieldChange
  end
  object CamCheck: TCheckBox
    Left = 8
    Top = 150
    Width = 420
    Height = 17
    Hint = '@MB:ASphericalPanoramaGetsTheEXIF'
    Caption = '@MB:N360PhotosTheCameraTagFacebook'
    Checked = True
    ParentShowHint = False
    ShowHint = True
    State = cbChecked
    TabOrder = 6
    OnClick = CamCheckClick
  end
  object Label4: TLabel
    Left = 8
    Top = 178
    Width = 80
    Height = 13
    Caption = '@MB:PostText'
  end
  object PostMemo: TMemo
    Left = 8
    Top = 196
    Width = 524
    Height = 150
    ScrollBars = ssVertical
    TabOrder = 7
  end
  object Label5: TLabel
    Left = 8
    Top = 362
    Width = 80
    Height = 13
    Caption = '@MB:Folder2'
  end
  object FolderEdit: TEdit
    Left = 96
    Top = 358
    Width = 400
    Height = 21
    TabOrder = 8
  end
  object FolderBtn: TButton
    Left = 500
    Top = 357
    Width = 32
    Height = 23
    Caption = '...'
    TabOrder = 9
    OnClick = FolderBtnClick
  end
  object MakeBtn: TButton
    Left = 8
    Top = 414
    Width = 130
    Height = 27
    Caption = '@MB:MakeTheFiles'
    Default = True
    TabOrder = 10
    OnClick = MakeBtnClick
  end
  object CopyBtn: TButton
    Left = 146
    Top = 414
    Width = 130
    Height = 27
    Caption = '@MB:CopyThePostText'
    TabOrder = 11
    OnClick = CopyBtnClick
  end
  object OpenBtn: TButton
    Left = 284
    Top = 414
    Width = 130
    Height = 27
    Caption = '@MB:OpenTheFolder'
    Enabled = False
    TabOrder = 12
    OnClick = OpenBtnClick
  end
  object CloseBtn: TButton
    Left = 422
    Top = 414
    Width = 110
    Height = 27
    Cancel = True
    Caption = '@MB:Close'
    TabOrder = 13
    OnClick = CloseBtnClick
  end
  object StatusLabel: TLabel
    Left = 8
    Top = 382
    Width = 524
    Height = 30
    AutoSize = False
    WordWrap = True
    Caption = ''
  end
end
