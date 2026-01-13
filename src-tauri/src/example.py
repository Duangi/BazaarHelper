from clip import crop_focus

# 1) 保留上半部分
crop_focus('./test.jpg', 'tmp_top.jpg', top_fraction=0.5, center_fraction=1.0)

# 2) 在上半部分内再取下方 60%
crop_focus('tmp_top.jpg', 'tmp_mid.jpg', top_fraction=0.7, keep='bottom', center_fraction=1.0)

# 3) 最后左右裁切，只留中间 1/3
crop_focus('tmp_mid.jpg', 'final.jpg', top_fraction=1.0, center_fraction=5/12)