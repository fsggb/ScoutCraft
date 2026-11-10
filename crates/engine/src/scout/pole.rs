//! Poles, boards and the operations that join and shape them. Lengths are millimetres, z is up.
//!
//! A [`Pole`] is a round pole (or, with a `width`, a board) along a straight line. Its own frame
//! has `z` along the pole, `x` its `up` (the side a notch or a split is cut from) and `y` across;
//! every [`Op`] is given in that frame, so a built pole is only ever *added to*: a tenon is a stub
//! joined to its end, a mortise or a notch is a cut, and the visible part of the pole is
//! `start .. start + length` (shortening moves `start` or `length`, never the anchor `origin`).
//!
//! The joint builders ([`Build::tenon`], [`Build::mibois`], [`Build::peg`], [`Build::lash`]) work
//! from the poles' world lines, so the same code makes the joints of a furniture template and the
//! joint a person asks for between two poles already in the design.

use serde::{Deserialize, Serialize};

pub type V3 = [f64; 3];

fn one() -> u8 {
    1
}

/// Most poles one design part may have (a hostile size cannot ask for millions).
pub const MAX_POLES: usize = 400;
/// Most operations one pole record may carry (the record is a property of 4 KB).
pub const MAX_OPS: usize = 14;
/// Play left in a mortise, in mm.
pub const JEU: f64 = 1.0;

pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub fn len(a: V3) -> f64 {
    dot(a, a).sqrt()
}
pub fn norm(a: V3) -> V3 {
    let l = len(a);
    if l < 1e-12 { [0.0, 0.0, 1.0] } else { mul(a, 1.0 / l) }
}
fn round3(a: V3) -> V3 {
    a.map(|x| (x * 1000.0).round() / 1000.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum End {
    Start,
    End,
}

/// The tenons of CathIA: the classic (hachette) and sawn round tenons, the flat tenon, the
/// dovetails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Style {
    /// Round tenon (chopped with a hatchet).
    Hache,
    /// Round tenon, sawn square at the shoulder.
    Scie,
    /// Flat (rectangular) tenon.
    Plat,
    /// Dovetail (queue d'aronde): a flat tenon that flares towards its tip.
    Aronde,
}

impl Style {
    pub fn label(self) -> &'static str {
        match self {
            Style::Hache => "tenon classique (hachette)",
            Style::Scie => "tenon scié",
            Style::Plat => "tenon plat",
            Style::Aronde => "queue d'aronde",
        }
    }
    pub fn round(self) -> bool {
        matches!(self, Style::Hache | Style::Scie)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitMode {
    /// Split lengthwise: the half on the `up` side is taken away (the flat face looks up).
    Demi,
    /// A quarter log: three quarters are taken away.
    Quart,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum Op {
    /// A stub joined to the end of the pole. `wide` is the local axis (0 = x, 1 = y) the flat
    /// tenon's width lies along.
    Tenon { end: End, style: Style, d: f64, w: f64, len: f64, wide: u8, theta: f64, #[serde(default = "one")] typ: u8 },
    /// A cut that takes the tenon: `c` is where it starts (outside the pole), `u` its direction.
    Mortise { c: V3, u: V3, style: Style, d: f64, w: f64, depth: f64, wide: u8, theta: f64, #[serde(default = "one")] typ: u8, #[serde(default)] open: f64 },
    /// A box cut centred on `c` with half sizes `h` (a half-lap notch).
    Notch { c: V3, h: V3 },
    /// A round hole (a peg's).
    Hole { c: V3, u: V3, d: f64, depth: f64 },
    Split { mode: SplitMode },
    /// Cut everything beyond local z = `at` at that end.
    CutOff { end: End, at: f64 },
    /// The end `end` is cut to fit the round pole `with` (`#n`: the pole of index n of the part, or the
    /// component's name): the pole is built `extra` mm longer and the other pole's body is cut from it.
    Saddle { with: String, end: End, extra: f64 },
    /// A sharpened end: the last `len` mm taper to a point.
    Point { end: End, len: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pole {
    pub role: String,
    /// Round: the diameter. Board: its thickness (along local x).
    pub diameter: f64,
    /// 0 for a round pole; a board's width (along local y).
    pub width: f64,
    /// Visible length, from `start`.
    pub length: f64,
    /// Local z of the visible start (the anchor `origin` is local z = 0).
    pub start: f64,
    pub origin: V3,
    pub dir: V3,
    pub up: V3,
    pub ops: Vec<Op>,
}

fn default_up(dir: V3) -> V3 {
    if dir[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] }
}

impl Pole {
    /// A round pole along `p → q`.
    pub fn bar(role: &str, diameter: f64, p: V3, q: V3) -> Pole {
        let d = sub(q, p);
        let dir = norm(d);
        Pole { role: role.to_string(), diameter, width: 0.0, length: len(d), start: 0.0, origin: p, dir, up: default_up(dir), ops: Vec::new() }
    }
    /// A board (`thickness` × `width`) along `p → q`, its thickness towards `up`.
    pub fn board(role: &str, thickness: f64, width: f64, p: V3, q: V3, up: V3) -> Pole {
        let mut b = Pole::bar(role, thickness, p, q);
        b.width = width;
        b.up = up;
        b
    }
    pub fn radius(&self) -> f64 {
        if self.width > 0.0 { self.diameter.max(self.width) / 2.0 } else { self.diameter / 2.0 }
    }
    pub fn x_axis(&self) -> V3 {
        let u = sub(self.up, mul(self.dir, dot(self.up, self.dir)));
        if len(u) < 1e-6 { norm(sub(default_up(self.dir), mul(self.dir, dot(default_up(self.dir), self.dir)))) } else { norm(u) }
    }
    pub fn y_axis(&self) -> V3 {
        cross(self.dir, self.x_axis())
    }
    /// World point → this pole's frame.
    pub fn local(&self, p: V3) -> V3 {
        let r = sub(p, self.origin);
        round3([dot(r, self.x_axis()), dot(r, self.y_axis()), dot(r, self.dir)])
    }
    pub fn local_dir(&self, d: V3) -> V3 {
        round3([dot(d, self.x_axis()), dot(d, self.y_axis()), dot(d, self.dir)])
    }
    /// The row-major 3×4 matrix that turns a pole built along local +z into this one.
    pub fn matrix(&self) -> [f64; 12] {
        let (x, y, z, o) = (self.x_axis(), self.y_axis(), self.dir, self.origin);
        [x[0], y[0], z[0], o[0], x[1], y[1], z[1], o[1], x[2], y[2], z[2], o[2]]
    }
    pub fn p_start(&self) -> V3 {
        add(self.origin, mul(self.dir, self.start))
    }
    pub fn end(&self) -> V3 {
        add(self.origin, mul(self.dir, self.start + self.length))
    }
    /// The local z range the body is built over: the visible part, longer at an end that is cut to fit
    /// another pole.
    pub fn built_range(&self) -> (f64, f64) {
        let (mut z0, mut z1) = (self.start, self.start + self.length);
        for o in &self.ops {
            if let Op::Saddle { end, extra, .. } = o {
                match end {
                    End::Start => z0 -= extra,
                    End::End => z1 += extra,
                }
            }
        }
        (z0, z1)
    }
    pub fn tenon_total(&self) -> f64 {
        self.ops.iter().map(|o| if let Op::Tenon { len, .. } = o { *len } else { 0.0 }).sum()
    }
    /// What it is cut to from the log: the visible length and its tenons.
    pub fn cut_length(&self) -> f64 {
        self.length + self.tenon_total()
    }
    /// Cut off `by` mm at an end (the visible part only: no operation).
    pub fn trim(&mut self, end: End, by: f64) {
        match end {
            End::End => self.length -= by,
            End::Start => {
                self.start += by;
                self.length -= by;
            }
        }
    }
    pub fn push(&mut self, op: Op) -> Result<(), String> {
        if self.ops.len() >= MAX_OPS {
            return Err(format!("« {} » a déjà {MAX_OPS} opérations : c'est le maximum", self.role));
        }
        self.ops.push(op);
        Ok(())
    }
    pub fn tenons(&self) -> impl Iterator<Item = &Op> {
        self.ops.iter().filter(|o| matches!(o, Op::Tenon { .. }))
    }
}

/// Closest points of two infinite lines `p + t·u` and `q + s·v`, with their parameters; `None`
/// when they are (nearly) parallel.
pub fn closest(p: V3, u: V3, q: V3, v: V3) -> Option<(V3, V3, f64, f64)> {
    let w = sub(p, q);
    let (a, b, c, d, e) = (dot(u, u), dot(u, v), dot(v, v), dot(u, w), dot(v, w));
    let den = a * c - b * b;
    if den.abs() < 1e-9 {
        return None;
    }
    let (t, s) = ((b * e - c * d) / den, (a * e - b * d) / den);
    Some((add(p, mul(u, t)), add(q, mul(v, s)), t, s))
}

/// The poles of a part under construction, and the joints made between them.
#[derive(Default)]
pub struct Build {
    pub poles: Vec<Pole>,
}

/// What a tenon asks for.
#[derive(Clone, Copy, Debug)]
pub struct TenonOpts {
    pub style: Style,
    pub through: bool,
    /// Tenon diameter or thickness (0: the usual one).
    pub d: f64,
    /// Flat tenon width (0: the usual one).
    pub w: f64,
    /// Dovetail flare, in degrees.
    pub theta: f64,
    /// Dovetail kind: 1 classic, 2 through (comes out the other side), 3 along the receiving pole's axis.
    pub typ: u8,
    /// Cut the tenon pole's end to the curve of the receiving round pole (a wolf's mouth): slow.
    pub saddle: bool,
}

impl TenonOpts {
    pub fn new(style: Style) -> Self {
        TenonOpts { style, through: false, d: 0.0, w: 0.0, theta: 15.0, typ: 1, saddle: false }
    }
    pub fn through(mut self) -> Self {
        self.through = true;
        self
    }
}

fn dominant(v: V3) -> usize {
    let a = v.map(f64::abs);
    if a[0] >= a[1] && a[0] >= a[2] { 0 } else if a[1] >= a[2] { 1 } else { 2 }
}

impl Build {
    pub fn bar(&mut self, role: &str, d: f64, p: V3, q: V3) -> usize {
        self.poles.push(Pole::bar(role, d, p, q));
        self.poles.len() - 1
    }
    pub fn board(&mut self, role: &str, t: f64, w: f64, p: V3, q: V3, up: V3) -> usize {
        self.poles.push(Pole::board(role, t, w, p, q, up));
        self.poles.len() - 1
    }

    /// Tenon of `t` into `r`: the end of `t` nearest `r` is trimmed to `r`'s surface and gets a
    /// tenon; `r` gets the mortise.
    pub fn tenon(&mut self, t: usize, r: usize, o: TenonOpts) -> Result<(), String> {
        let (pt, pr) = match (self.poles.get(t), self.poles.get(r)) {
            (Some(a), Some(b)) if t != r => (a.clone(), b.clone()),
            _ => return Err("choisissez deux pièces différentes".into()),
        };
        let (ct, cr, tc, _) = closest(pt.origin, pt.dir, pr.origin, pr.dir)
            .ok_or_else(|| format!("« {} » et « {} » sont parallèles : un tenon demande qu'elles se croisent", pt.role, pr.role))?;
        let (s0, s1) = (pt.start, pt.start + pt.length);
        let end = if (tc - s0).abs() < (tc - s1).abs() { End::Start } else { End::End };
        let sin = len(cross(pt.dir, pr.dir)).max(0.35);
        // How far inside `r` its surface is along `t`: a tenon off `r`'s axis meets it sooner.
        let off = len(sub(ct, cr));
        let r_in = (pr.radius() * pr.radius() - off * off).max(0.09 * pr.radius() * pr.radius()).sqrt();
        let rr = r_in / sin;
        let (dt, dr) = (pt.diameter.min(if pt.width > 0.0 { pt.width } else { pt.diameter }), pr.diameter);
        // Where the tip is, and where the surface of `r` is, along `t`.
        let (tip, surf) = if end == End::End { (s1, tc - rr) } else { (s0, tc + rr) };
        let overlap = if end == End::End { tip - surf } else { surf - tip };
        if overlap > 120.0 {
            return Err(format!("« {} » dépasse de {} mm dans « {} » : reculez-la pour qu'elle n'y entre que de quelques millimètres", pt.role, overlap.round(), pr.role));
        }
        if (tip - tc).abs() > rr + 120.0 && overlap < -120.0 {
            return Err(format!("« {} » est trop loin de « {} » : rapprochez un bout de l'autre pièce", pt.role, pr.role));
        }
        let default_d = (dt * 0.55).min(dr / 3.0).max(8.0).round();
        let d = if o.d > 0.0 { o.d } else if o.style.round() { default_d } else { (dt * 0.6).min(dr / 3.0 + 8.0).max(8.0).round() };
        let w = if o.w > 0.0 { o.w } else { (dt * 0.7).max(15.0).round() };
        let u_world = if end == End::End { pt.dir } else { mul(pt.dir, -1.0) };
        // Where the tenon enters `r`, and how much wood it finds there: a split log has a flat face.
        let entry = add(pt.origin, mul(pt.dir, surf));
        let chord = 2.0 * rr;
        let mut avail = chord;
        let (ul, el) = (pr.local_dir(u_world), pr.local(entry));
        for op in &pr.ops {
            if let Op::Split { mode } = op {
                if ul[0] > 1e-6 && el[0] < 0.0 {
                    avail = avail.min(-el[0] / ul[0]);
                }
                if *mode == SplitMode::Quart && ul[1] > 1e-6 && el[1] < 0.0 {
                    avail = avail.min(-el[1] / ul[1]);
                }
            }
        }
        if avail < 12.0 {
            return Err(format!("« {} » est trop mince, ou fendue, à cet endroit pour recevoir un tenon de « {} »", pr.role, pt.role));
        }
        let aronde = o.style == Style::Aronde;
        let through = (o.through || (aronde && o.typ == 2)) && avail >= chord - 0.5;
        let tlen = if through { chord + 6.0 } else { (0.6 * avail).min(avail - 4.0).max(12.0).round() };
        // A dovetail flares towards its tip, but never beyond the pole it is cut in.
        let mut theta = if o.theta > 0.0 { o.theta } else { 20.0 };
        if aronde {
            let tip = w + 2.0 * tlen * theta.to_radians().tan();
            let limit = 0.92 * dt;
            if tip > limit {
                theta = (((limit - w) / (2.0 * tlen)).max(0.0875)).atan().to_degrees();
            }
        }
        // Trim to the surface; the stub goes in.
        {
            let p = &mut self.poles[t];
            match end {
                End::End => p.length = (surf - p.start).max(1.0),
                End::Start => {
                    let by = (surf - p.start).max(0.0);
                    p.trim(End::Start, by);
                }
            }
        }
        let ptn = self.poles[t].clone();
        // The width lies along whichever of the tenon pole's x or y is nearer `r`'s axis (a dovetail
        // along the axis of `r` lies across it).
        let rl = ptn.local_dir(pr.dir);
        let near = if rl[0].abs() >= rl[1].abs() { 0 } else { 1 };
        let wide = if aronde && o.typ == 3 { 1 - near } else { near };
        let typ = if aronde { o.typ.clamp(1, 3) } else { 1 };
        self.poles[t].push(Op::Tenon { end, style: o.style, d, w, len: tlen, wide, theta, typ })?;
        if o.saddle && pr.width == 0.0 && ptn.width == 0.0 {
            // the end of `t` is cut to the curve of `r`, so the two meet without a gap or an overlap
            let rt = ptn.radius();
            let big_r = pr.radius();
            let far = (big_r * big_r - (off.abs() + rt).powi(2)).max(0.0).sqrt();
            // never longer than the wood there is: an end that came out of a split log's flat face
            // would be left as a loose disc
            let extra = ((r_in - far) / sin + 2.0).min(chord * 0.9).min(avail - 3.0).max(0.0);
            self.poles[t].push(Op::Saddle { with: format!("#{r}"), end, extra })?;
        }
        // The mortise: on `t`'s line, from just outside the surface of `r`.
        let base = sub(entry, mul(u_world, 3.0));
        let depth = if through { chord + 10.0 } else { tlen + JEU + 3.0 };
        let (c, u) = (self.poles[r].local(base), self.poles[r].local_dir(u_world));
        let (mw, md) = if o.style.round() { (d + JEU, d + JEU) } else { (w + JEU, d + JEU) };
        // A dovetail's groove runs the whole width of `r` along the slide direction.
        let open = if aronde { 2.0 * pr.radius() + 20.0 } else { 0.0 };
        self.poles[r].push(Op::Mortise { c, u, style: o.style, d: md, w: mw, depth, wide, theta, typ, open })?;
        Ok(())
    }

    /// Half-lap of `a` and `b` (a takes `pc` % of the joint's thickness, from above; b the rest).
    pub fn mibois(&mut self, a: usize, b: usize, pc: f64) -> Result<(), String> {
        let (pa, pb) = match (self.poles.get(a), self.poles.get(b)) {
            (Some(x), Some(y)) if a != b => (x.clone(), y.clone()),
            _ => return Err("choisissez deux pièces différentes".into()),
        };
        let mut k = cross(pa.dir, pb.dir);
        if len(k) < 0.25 {
            return Err("les deux pièces sont presque parallèles : le mi-bois demande qu'elles se croisent en angle".into());
        }
        k = norm(k);
        let sin = len(cross(pa.dir, pb.dir));
        let (ca, cb, ta, tb) = closest(pa.origin, pa.dir, pb.origin, pb.dir).ok_or("pièces parallèles")?;
        if dot(sub(cb, ca), k) < 0.0 {
            k = mul(k, -1.0);
        }
        // Coordinates along k of the faces: a is below b.
        let (ka, kb) = (dot(ca, k), dot(cb, k));
        let (a_hi, b_lo) = (ka + pa.radius(), kb - pb.radius());
        let (a_lo, b_hi) = (ka - pa.radius(), kb + pb.radius());
        let _ = (a_lo, b_hi);
        if a_hi - b_lo < 1.0 {
            return Err("les deux pièces doivent se chevaucher : posez l'une sur l'autre (elles se recouvrent de la profondeur de l'entaille)".into());
        }
        for (t, p) in [(ta, &pa), (tb, &pb)] {
            if t < p.start - p.radius() || t > p.start + p.length + p.radius() {
                return Err(format!("« {} » ne passe pas par la jonction", p.role));
            }
        }
        let pc = (pc / 100.0).clamp(0.1, 0.9);
        let m = a_hi - pc * (a_hi - b_lo);
        let notch = |p: &Pole, other: &Pole, lo: f64, hi: f64, at: V3| -> Op {
            // Centre of the notch: on the pole's line at the crossing, moved along k to the middle of lo..hi.
            let kc = dot(at, k);
            let centre = add(at, mul(k, (lo + hi) / 2.0 - kc));
            let c = p.local(centre);
            // Half sizes: along k (depth), across (the pole's full width), along the pole (the other's width).
            let along = other.radius() / sin + JEU / 2.0;
            let depth = (hi - lo) / 2.0 + 1.5;
            let across = p.radius() + 2.0;
            let kl = p.local_dir(k);
            let mut h = [0.0; 3];
            let dk = dominant(kl);
            h[2] = along;
            h[dk] = depth;
            // The remaining local axis (x or y) is the across one.
            let other_axis = if dk == 0 { 1 } else { 0 };
            if dk == 2 {
                h = [across, across, depth];
            } else {
                h[other_axis] = across;
            }
            Op::Notch { c, h }
        };
        let na = notch(&pa, &pb, m, a_hi + 3.0, ca);
        let nb = notch(&pb, &pa, b_lo - 3.0, m, cb);
        self.poles[a].push(na)?;
        self.poles[b].push(nb)?;
        Ok(())
    }

    /// A peg of diameter `d` through `a` and `b` where they cross (along the stacking direction
    /// for crossing poles, else across `a`). Returns the peg's index.
    /// A mortise through `r` only along `t`'s line (a pole that passes through another).
    pub fn through(&mut self, t: usize, r: usize) -> Result<(), String> {
        let (pt, pr) = match (self.poles.get(t), self.poles.get(r)) {
            (Some(a), Some(b)) if t != r => (a.clone(), b.clone()),
            _ => return Err("choisissez deux pièces différentes".into()),
        };
        let (ct, _, _, _) = closest(pt.origin, pt.dir, pr.origin, pr.dir).ok_or("pièces parallèles")?;
        let reach = pr.radius() * 2.0 / len(cross(pt.dir, pr.dir)).max(0.35) + 20.0;
        let base = sub(ct, mul(pt.dir, reach / 2.0));
        let (c, u) = (pr.local(base), pr.local_dir(pt.dir));
        self.poles[r].push(Op::Mortise { c, u, style: Style::Hache, d: pt.diameter + JEU, w: pt.diameter + JEU, depth: reach, wide: 0, theta: 0.0, typ: 1, open: 0.0 })
    }

    /// A peg of diameter `d` through the point `p` along `k`, through `a` and `b`.
    pub fn peg_line(&mut self, a: usize, b: usize, p: V3, k: V3, d: f64, total: f64) -> Result<usize, String> {
        let k = norm(k);
        let origin = sub(p, mul(k, total / 2.0));
        let hole = |q: &Pole| -> Op {
            let base = sub(p, mul(k, total / 2.0 + 2.0));
            Op::Hole { c: q.local(base), u: q.local_dir(k), d: d + JEU, depth: total + 4.0 }
        };
        let (ha, hb) = match (self.poles.get(a), self.poles.get(b)) {
            (Some(x), Some(y)) => (hole(x), hole(y)),
            _ => return Err("pièce inconnue".into()),
        };
        self.poles[a].push(ha)?;
        self.poles[b].push(hb)?;
        let mut peg = Pole::bar("cheville", d, origin, add(origin, mul(k, total)));
        peg.up = default_up(peg.dir);
        self.poles.push(peg);
        Ok(self.poles.len() - 1)
    }

    pub fn peg(&mut self, a: usize, b: usize, d: f64) -> Result<usize, String> {
        let (pa, pb) = match (self.poles.get(a), self.poles.get(b)) {
            (Some(x), Some(y)) if a != b => (x.clone(), y.clone()),
            _ => return Err("choisissez deux pièces différentes".into()),
        };
        let k = {
            let c = cross(pa.dir, pb.dir);
            if len(c) < 0.25 { pa.x_axis() } else { norm(c) }
        };
        let (ca, cb, _, _) = closest(pa.origin, pa.dir, pb.origin, pb.dir).unwrap_or((pa.origin, pb.origin, 0.0, 0.0));
        let mid = mul(add(ca, cb), 0.5);
        let total = pa.radius() + pb.radius() + 2.0 * JEU + 16.0 + dot(sub(cb, ca), k).abs();
        self.peg_line(a, b, mid, k, d, total)
    }

    /// Lashing where `a` and `b` cross: `turns` turns of rope of diameter `rope`, as two rings.
    pub fn lash(&mut self, a: usize, b: usize, turns: f64, rope: f64) -> Result<(), String> {
        let (pa, pb) = match (self.poles.get(a), self.poles.get(b)) {
            (Some(x), Some(y)) if a != b => (x.clone(), y.clone()),
            _ => return Err("choisissez deux pièces différentes".into()),
        };
        let (ca, cb, _, _) = closest(pa.origin, pa.dir, pb.origin, pb.dir).ok_or("les deux pièces sont parallèles : un brêlage demande qu'elles se croisent")?;
        let h = (turns * rope).clamp(rope, 200.0);
        for (p, c) in [(&pa, ca), (&pb, cb)] {
            let o = sub(c, mul(p.dir, h / 2.0));
            let mut ring = Pole::bar("brêlage", p.radius() * 2.0 + 2.0 * rope, o, add(o, mul(p.dir, h)));
            ring.width = 0.0;
            ring.ops.push(Op::Hole { c: [0.0, 0.0, -1.0], u: [0.0, 0.0, 1.0], d: p.radius() * 2.0, depth: h + 2.0 });
            self.poles.push(ring);
        }
        Ok(())
    }

    pub fn split(&mut self, i: usize, mode: SplitMode) -> Result<(), String> {
        self.poles.get_mut(i).ok_or("pièce inconnue")?.push(Op::Split { mode })
    }
}

/// Total length of wood in metres.
pub fn wood_metres(poles: &[Pole]) -> f64 {
    poles.iter().filter(|p| p.role != "brêlage" && p.role != "ficelle").map(Pole::cut_length).sum::<f64>() / 1000.0
}

/// The overall size (x, y, z) of a set of poles.
pub fn extent(poles: &[Pole]) -> [f64; 3] {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in poles {
        let r = p.radius();
        for q in [p.p_start(), p.end()] {
            for i in 0..3 {
                // The pole's thickness spreads across the axes it does not run along.
                let spread = r * (1.0 - p.dir[i] * p.dir[i]).max(0.0).sqrt();
                lo[i] = lo[i].min(q[i] - spread);
                hi[i] = hi[i].max(q[i] + spread);
            }
        }
    }
    if poles.is_empty() {
        return [0.0; 3];
    }
    [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]]
}

/// The cut list: poles of one role, section and length together, in the order they first appear.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CutRow {
    pub role: String,
    pub diameter: f64,
    pub width: f64,
    pub length: f64,
    pub count: usize,
    /// Length to cut from the log, tenons included.
    pub cut_length: f64,
    pub tenons: usize,
}

pub fn cut_list(poles: &[Pole]) -> Vec<CutRow> {
    let mut rows: Vec<CutRow> = Vec::new();
    for p in poles {
        let (l, cl) = ((p.length * 10.0).round() / 10.0, (p.cut_length() * 10.0).round() / 10.0);
        match rows.iter_mut().find(|r| r.role == p.role && r.diameter == p.diameter && r.width == p.width && r.length == l && r.cut_length == cl) {
            Some(r) => r.count += 1,
            None => rows.push(CutRow { role: p.role.clone(), diameter: p.diameter, width: p.width, length: l, count: 1, cut_length: cl, tenons: p.tenons().count() }),
        }
    }
    rows
}

/// The joints of a set of poles for the plan: (what, how many).
pub fn joint_summary(poles: &[Pole]) -> Vec<(String, usize)> {
    let mut v: Vec<(String, usize)> = Vec::new();
    let mut bump = |k: String| match v.iter_mut().find(|(n, _)| *n == k) {
        Some((_, c)) => *c += 1,
        None => v.push((k, 1)),
    };
    for p in poles {
        for o in &p.ops {
            match o {
                Op::Tenon { style, d, w, .. } => bump(if style.round() { format!("{} Ø{} mm", style.label(), d) } else { format!("{} {}×{} mm", style.label(), w, d) }),
                Op::Notch { .. } => bump("mi-bois (deux entailles)".into()),
                Op::Split { mode } => bump(if *mode == SplitMode::Demi { "rondin fendu en deux".into() } else { "quart de bois".into() }),
                Op::Point { .. } => bump("pointe".into()),
                _ => {}
            }
        }
        if p.role == "cheville" {
            bump(format!("cheville Ø{} mm", p.diameter));
        }
        if p.role == "brêlage" {
            bump("brêlage".into());
        }
    }
    // A mi-bois is two notches: count joints, not notches.
    for (k, c) in v.iter_mut() {
        if k.starts_with("mi-bois") {
            *c = (*c).div_ceil(2);
            *k = "mi-bois".into();
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tee() -> Build {
        let mut b = Build::default();
        // a post and a rail whose end reaches the post's axis
        b.bar("pied", 80.0, [0.0, 0.0, 0.0], [0.0, 0.0, 700.0]);
        b.bar("traverse", 60.0, [0.0, 0.0, 600.0], [1000.0, 0.0, 600.0]);
        b
    }

    #[test]
    fn the_frame_of_a_pole_is_orthonormal_and_round_trips() {
        let p = Pole::bar("x", 50.0, [10.0, 20.0, 30.0], [10.0, 520.0, 30.0]);
        let (x, y, z) = (p.x_axis(), p.y_axis(), p.dir);
        assert!(dot(x, y).abs() < 1e-9 && dot(x, z).abs() < 1e-9 && dot(y, z).abs() < 1e-9);
        let w = [10.0, 270.0, 30.0];
        assert_eq!(p.local(w), [0.0, 0.0, 250.0]);
        let m = p.matrix();
        assert_eq!(&m[3..4], &[10.0]);
    }

    #[test]
    fn a_tenon_trims_the_rail_and_cuts_a_mortise() {
        let mut b = tee();
        b.tenon(1, 0, TenonOpts::new(Style::Hache)).unwrap_or_default();
        let rail = &b.poles[1];
        assert!((rail.start - 40.0).abs() < 1e-6, "{}", rail.start);
        assert!(rail.tenons().count() == 1);
        assert!(matches!(b.poles[0].ops.first(), Some(Op::Mortise { .. })));
        assert!(rail.cut_length() > rail.length);
    }

    #[test]
    fn a_tenon_between_parallel_poles_is_refused() {
        let mut b = Build::default();
        b.bar("a", 50.0, [0.0, 0.0, 0.0], [0.0, 0.0, 500.0]);
        b.bar("b", 50.0, [100.0, 0.0, 0.0], [100.0, 0.0, 500.0]);
        assert!(b.tenon(0, 1, TenonOpts::new(Style::Scie)).is_err());
        assert!(b.tenon(0, 0, TenonOpts::new(Style::Scie)).is_err());
        assert!(b.tenon(0, 9, TenonOpts::new(Style::Scie)).is_err());
    }

    #[test]
    fn a_half_lap_notches_both_poles() {
        let mut b = Build::default();
        // two poles crossing, one lying on the other
        b.bar("a", 80.0, [0.0, 0.0, 0.0], [1000.0, 0.0, 0.0]);
        b.bar("b", 80.0, [500.0, -500.0, 70.0], [500.0, 500.0, 70.0]);
        b.mibois(0, 1, 50.0).unwrap_or_default();
        assert!(matches!(b.poles[0].ops.first(), Some(Op::Notch { .. })) && matches!(b.poles[1].ops.first(), Some(Op::Notch { .. })));
        assert_eq!(joint_summary(&b.poles), vec![("mi-bois".to_string(), 1)]);
        // too far apart: refused
        let mut far = Build::default();
        far.bar("a", 80.0, [0.0, 0.0, 0.0], [1000.0, 0.0, 0.0]);
        far.bar("b", 80.0, [500.0, -500.0, 500.0], [500.0, 500.0, 500.0]);
        assert!(far.mibois(0, 1, 50.0).is_err());
    }

    #[test]
    fn a_peg_adds_a_pole_and_two_holes() {
        let mut b = Build::default();
        b.bar("a", 80.0, [0.0, 0.0, 0.0], [1000.0, 0.0, 0.0]);
        b.bar("b", 80.0, [500.0, -500.0, 70.0], [500.0, 500.0, 70.0]);
        let n = b.peg(0, 1, 20.0).unwrap_or(99);
        assert_eq!(n, 2);
        assert_eq!(b.poles[2].role, "cheville");
        assert!(matches!(b.poles[0].ops.first(), Some(Op::Hole { .. })));
    }

    #[test]
    fn the_op_record_stays_small_and_round_trips() {
        let mut p = Pole::bar("pied", 80.0, [0.0; 3], [0.0, 0.0, 700.0]);
        for _ in 0..MAX_OPS {
            p.push(Op::Notch { c: [1.0, 2.0, 3.0], h: [4.0, 5.0, 6.0] }).unwrap_or_default();
        }
        assert!(p.push(Op::Split { mode: SplitMode::Demi }).is_err());
        let text = serde_json::to_string(&p).unwrap_or_default();
        assert!(text.len() < 3000, "{}", text.len());
        assert_eq!(serde_json::from_str::<Pole>(&text).ok(), Some(p));
    }
}
