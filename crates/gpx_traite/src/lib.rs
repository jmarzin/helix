#[macro_use]
extern crate helix;
extern crate quick_xml;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use quick_xml::Reader;
use quick_xml::events::Event;
use std::time::Instant;

#[derive(Debug, Copy, Clone)]
struct Point {
    lat: f64,
    lon: f64,
    ele: f64,
}
#[derive(Serialize, Deserialize, Debug)]
struct Resultat {
    heure_debut : String,
    heure_fin : String,
    lon_depart: f64,
    lat_depart: f64,
    lon_arrivee: f64,
    lat_arrivee: f64,
    altitude_mini : f64,
    altitude_maxi : f64,
    cumul_montee: f64,
    cumul_descente: f64,
    distance: f64,
    profil: Vec<Vec<[i32 ; 2]>>
}

fn lit_fichier(nom: &String) -> (String, String, Vec<Point>) {
    let mut reader = Reader::from_file(nom).unwrap();
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut points: Vec<Point> = Vec::new();
    let mut point = Point { lon: 0f64, lat: 0f64, ele: 0f64 };
    let mut dans_trkpt = false;
    let mut dans_ele = false;
    let mut dans_time = false;
    let mut heure_debut = None;
    let mut heure_fin = "".to_string();

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"trkpt" => {
                        dans_trkpt = true;
                        point = Point { lon: 0f64, lat: 0f64, ele: 0f64 };
                        for att in e.attributes() {
                            let item = att.unwrap();
                            let cle = item.key.to_vec();
                            let cle = String::from_utf8(cle).unwrap();
                            let valeur = item.value.to_vec();
                            let valeur = String::from_utf8(valeur).unwrap().parse().unwrap();
                            if cle == "lon" {
                                point.lon = valeur
                            } else if cle == "lat" {
                                point.lat = valeur
                            }
                        }
                    }
                    b"ele" => dans_ele = true,
                    b"time" => dans_time = true,
                    _ => (),
                }
            }
            // unescape and decode the text event using the reader encoding
            Ok(Event::End(ref e)) => {
                match e.name() {
                    b"ele" => dans_ele = false,
                    b"time" => dans_time = false,
                    b"trkpt" => {
                        dans_trkpt = false;
                        points.push(point.clone());
                    }
                    _ => (),
                }
            }
            Ok(Event::Text(e)) => {
                if dans_ele && dans_trkpt {
                    point.ele = e.unescape_and_decode(&reader).unwrap().parse().unwrap()
                } else if dans_time && dans_trkpt {
                    if heure_debut.is_none() {
                        heure_debut = Some(e.unescape_and_decode(&reader).unwrap())
                    };
                    heure_fin = e.unescape_and_decode(&reader).unwrap()
                }
            }
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }

        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
    (heure_debut.unwrap(), heure_fin, points)
}

fn traite_altitudes(points: &Vec<Point>) -> (f64, f64, f64, f64, Vec<f64>) {
    let mut altitudes_lissees = Vec::new();
    let longueur = points.len();

    if longueur > 5 {
        altitudes_lissees.push(points[0].ele);
        altitudes_lissees.push(points[1].ele);
        for i in 2..longueur - 3 {
            let a_moy = (points[i - 2].ele
                + points[i - 1].ele
                + points[i].ele
                + points[i + 1].ele
                + points[i + 2].ele) / 5f64;
            altitudes_lissees.push(a_moy);
        }
        altitudes_lissees.push(points[longueur - 2].ele);
        altitudes_lissees.push(points[longueur - 1].ele);
    } else {
        for i in 0..longueur - 1 {
            altitudes_lissees.push(points[i].ele)
        }
    };
    let mut altitude_mini = 10_000f64;
    let mut altitude_maxi = -10_000f64;
    for i in 0..altitudes_lissees.len() - 1 {
        let a = altitudes_lissees[i];
        if a > altitude_maxi { altitude_maxi = a };
        if a < altitude_mini { altitude_mini = a };
    }
    let mut cumul_montee = 0.0;
    let mut cumul_descente = 0.0;
    for i in 1..altitudes_lissees.len() - 1 {
        let diff = altitudes_lissees[i] - altitudes_lissees[i - 1];
        if diff < 0.0 {
            cumul_descente += -diff;
        } else {
            cumul_montee += diff;
        }
    }

    (altitude_mini, altitude_maxi, cumul_montee, cumul_descente, altitudes_lissees)
}

fn calcule_distance(p1_lat: f64, p1_lon: f64, p2_lat: f64, p2_lon: f64) -> f64 {
    let a = 6_378_137.0;
    let b = 6_356_752.314245;
    let f = 1.0 / 298.257223563;
    let l_maj = (p2_lon - p1_lon).to_radians();
    let u_maj1 = ((1.0 - f) * p1_lat.to_radians().tan()).atan();
    let u_maj2 = ((1.0 - f) * p2_lat.to_radians().tan()).atan();
    let sin_u_maj1 = u_maj1.sin();
    let cos_u_maj1 = u_maj1.cos();
    let sin_u_maj2 = u_maj2.sin();
    let cos_u_maj2 = u_maj2.cos();
    let cos_sq_alpha = 0.0;
    let mut sin_sigma;
    let cos2_sigma_m = 0.0;
    let mut cos_sigma ;
    let mut sigma ;
    let mut lambda = l_maj;
    let mut iter_limit = 100;
    loop {
        let sin_lambda = lambda.sin();
        let cos_lambda = lambda.cos();
        sin_sigma = ((cos_u_maj2 * sin_lambda) * (cos_u_maj2 *
            sin_lambda) + (cos_u_maj1 * sin_u_maj2 -
            sin_u_maj1 * cos_u_maj2 * cos_lambda) *
            (cos_u_maj1 * sin_u_maj2 - sin_u_maj1 *
                cos_u_maj2 * cos_lambda)).sqrt();
        if sin_sigma == 0.0 { return 0.0};
        cos_sigma = sin_u_maj1 * sin_u_maj2 + cos_u_maj1 * cos_u_maj2 * cos_lambda;
        sigma = sin_sigma.atan2(cos_sigma);
        let sin_alpha = cos_u_maj1 * cos_u_maj2 * sin_lambda / sin_sigma;
        let cos_sq_alpha = 1.0 - sin_alpha * sin_alpha;
        let cos2_sigma_m = cos_sigma - 2.0 * sin_u_maj1 * sin_u_maj2 / cos_sq_alpha;
        let c_maj = f / 16.0 * cos_sq_alpha * (4.0 + f * (4.0 - 3.0 * cos_sq_alpha));
        let lambda_p = lambda;
        lambda = l_maj + (1.0 - c_maj) * f * sin_alpha *
            (sigma + c_maj * sin_sigma * (cos2_sigma_m + c_maj *
                cos_sigma * (-1.0 + 2.0 * cos2_sigma_m * cos2_sigma_m)));
        iter_limit -= 1;
        if (lambda - lambda_p).abs() < 1e-12 || iter_limit <= 0 { break };
    }
    if iter_limit == 0 { return 0.0 };
    let u_sq = cos_sq_alpha * (a * a - b * b) / (b * b);
    let a_maj = 1.0 + u_sq / 16_384.0 * (4096.0 + u_sq *
        (-768.0 + u_sq * (320.0 - 175.0 * u_sq)));
    let b_maj = u_sq / 1024.0 * (256.0 + u_sq * (-128.0 + u_sq * (74.0 - 47.0 * u_sq)));
    let delta_sigma = b_maj * sin_sigma * (cos2_sigma_m + b_maj / 4.0 *
        (cos_sigma * (-1.0 + 2.0 * cos2_sigma_m * cos2_sigma_m) -
            b_maj / 6.0 * cos2_sigma_m * (-3.0 + 4.0 * sin_sigma *
                sin_sigma) * (-3.0 + 4.0 * cos2_sigma_m * cos2_sigma_m)));
    b * a_maj * (sigma - delta_sigma)
}

fn traite_distances(points: Vec<Point>) -> Vec<f64> {
    let mut distances_cumulees = Vec::new();
    let mut cumul = 0.0;
    distances_cumulees.push(cumul);
    for i in 1..points.len() - 1 {
        cumul += calcule_distance(points[i - 1].lat, points[i - 1].lon, points[i].lat, points[i].lon);
        distances_cumulees.push(cumul);
    }
    distances_cumulees
}

fn construit_profil(altitudes_lissees: &Vec<f64>, altitude_mini: f64, altitude_maxi: f64, distances_cumulees: Vec<f64>, distance: f64) -> Vec<Vec<[i32 ; 2]>> {
    let coef_x = 2000.0/distance;
    let coef_y = 2000.0/(altitude_maxi-altitude_mini);
    let mut profil = Vec::new();
    for i in 0..altitudes_lissees.len() - 1 {
        profil.push([(distances_cumulees[i] * coef_x).round() as i32, ((altitudes_lissees[i] - altitude_mini) * coef_y).round() as i32])
    };
    //3462
    let mut profil_filtre = Vec::new();
    let mut prec = [-1, -1];
    for el in profil {
        if el != prec {
            profil_filtre.push(el)
        }
        prec = el;
    };
    let mut profil = Vec::new();
    profil.push(profil_filtre);
    profil
}

ruby! {
    class GpxTraite {
        def traite_une_trace(nom: String) -> String {
            let r = lit_fichier(&nom);
            let points = r.2;
            let mut resultat = Resultat { heure_debut: r.0 , heure_fin: r.1,
                lon_depart: points.first().unwrap().lon, lat_depart: points.first().unwrap().lat,
                lon_arrivee: points.last().unwrap().lon, lat_arrivee: points.last().unwrap().lat,
                altitude_mini: 0.0, altitude_maxi: 0.0, cumul_montee: 0.0, cumul_descente: 0.0,
                distance: 0.0,
                profil: vec![]
            };

            let r = traite_altitudes(&points);
            resultat.altitude_mini = r.0;
            resultat.altitude_maxi = r.1;
            resultat.cumul_montee = r.2;
            resultat.cumul_descente = r.3;
            let altitudes_lissees = r.4;
            let distances_cumulees = traite_distances(points);
            resultat.distance = *distances_cumulees.last().unwrap();
            resultat.profil = construit_profil(&altitudes_lissees, resultat.altitude_mini, resultat.altitude_maxi, distances_cumulees, resultat.distance);
            serde_json::to_string(&resultat).unwrap()

        }
    }
}