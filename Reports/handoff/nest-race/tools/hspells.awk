# hspells.awk -- every hungry spell of every ant from digrows.csv (Deep trace, 2026-10-07).
# A spell starts at an ant's first decision row under half its grant (energy < 0.5, lean: NEEDS_FIRST's hungry line and
# where the game calls an ant too hungry to dig) and runs until its next row at or above the grant ('fed'), so the noise of
# digestion round the grant is not a spell. Spells still open when the rows stop are 'open' (the reader joins the ledger).
# Columns used: 1 frame, 2 id, 3 age, 4 worker, 5 hx, 6 hy, 7 zone, 9 hold, 17 energy, 24 outcome, 35 pull_why,
# 43 ants8, 46 way_d. Door columns 253-259 (the founding shaft, second_way.py's DOOR).
BEGIN { FS = ","; OFS = ","
  print "id,start,start_age,worker,start_zone,start_hx,start_hy,start_e,start_hold,end,last_row,how,min_e,rows,rows_nest,rows_out,left,fell,idle,stepped,p_store,p_hout,p_soil,p_none,p_ns,p_other,h0,h1,h2,maxdepth,door_rows,a8sum,last_hx,last_hy,last_zone,last_pull,last_hold,last_e,gains,gain_nest,first_gain_f,first_gain_x,first_gain_y,food_ahead,food_ahead_seen" }
NR == 1 { next }
{
  id = $2; f = $1 + 0; e = $17 + 0
  if (e < 1.0 && ((id in st) || e < 0.5)) {
    if (!(id in st)) {
      st[id] = f; sage[id] = $3; swk[id] = $4; sz[id] = $7; sx[id] = $5; sy[id] = $6; se[id] = e; sh[id] = $9
      mine[id] = e; n[id] = 0; nn[id] = 0; no[id] = 0; fe[id] = 0; id_[id] = 0; stp[id] = 0
      ps[id] = 0; ph[id] = 0; pw[id] = 0; pn[id] = 0; pq[id] = 0; po[id] = 0; h0[id] = 0; h1[id] = 0; h2[id] = 0
      md[id] = 0; dr[id] = 0; a8[id] = 0; gn[id] = 0; gnn[id] = 0; gf[id] = ""; gx[id] = ""; gy[id] = ""; fa[id] = 0; fs[id] = 0
    }
    n[id]++
    if (e < mine[id]) mine[id] = e
    if ($7 == "nest") nn[id]++; else no[id]++
    if ($24 == "fell") fe[id]++; else if ($24 == "roll_failed_idle") id_[id]++; else if ($24 == "stepped") stp[id]++
    p = $35
    if (p == "nest store") ps[id]++; else if (p == "hungry out") ph[id]++; else if (p == "soil way out") pw[id]++; else if (p == "none") pn[id]++; else if (p == "not scored") pq[id]++; else po[id]++
    if ($9 == "0") h0[id]++; else if ($9 == "1") h1[id]++; else h2[id]++
    y = $6 + 0; if (y > md[id]) md[id] = y
    x = $5 + 0; if (x >= 253 && x <= 259) dr[id]++
    a8[id] += $43 + 0
    if ($10 == "food") { fa[id]++; if ($15 + 0 > 0) fs[id]++ }
    if ((id in ph_) && ph_[id] != "1" && $9 == "1") { gn[id]++; if ($7 == "nest") gnn[id]++; if (gf[id] == "") { gf[id] = f; gx[id] = $5; gy[id] = $6 } }
    lf[id] = f; lx[id] = $5; ly[id] = $6; lz[id] = $7; lp[id] = $35; lh[id] = $9; le[id] = e
  } else if (id in st) {
    emit(id, f, "fed"); delete st[id]
  }
  ph_[id] = $9
}
function emit(id, endf, how) {
  print id, st[id], sage[id], swk[id], sz[id], sx[id], sy[id], se[id], sh[id], endf, lf[id], how, mine[id], n[id], nn[id], no[id], (no[id] > 0 ? 1 : 0), fe[id], id_[id], stp[id], ps[id], ph[id], pw[id], pn[id], pq[id], po[id], h0[id], h1[id], h2[id], md[id], dr[id], a8[id], lx[id], ly[id], lz[id], lp[id], lh[id], le[id], gn[id], gnn[id], gf[id], gx[id], gy[id], fa[id], fs[id]
}
END { for (id in st) emit(id, lf[id], "open") }
