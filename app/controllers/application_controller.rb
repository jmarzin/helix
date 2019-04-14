class ApplicationController < ActionController::Base
  def index
    debut = Time.now
    @resultat = GpxTraite.traite_une_trace(Rails.root.join("public", "gpx", "essai.GPX").to_s)
    fin = Time.now
    @temps = fin - debut
  end
end
